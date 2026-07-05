use anyhow::{Context, Result};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::warn;

use crate::oauth::MetaCredentials;
use crate::upload_state;

#[allow(dead_code)]
const DEFAULT_BASE_URL: &str = "https://graph.facebook.com/v22.0";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Instagram,
    Facebook,
}

impl Platform {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Instagram => "instagram",
            Platform::Facebook => "facebook",
        }
    }
}

impl std::str::FromStr for Platform {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "instagram" => Ok(Platform::Instagram),
            "facebook" => Ok(Platform::Facebook),
            _ => anyhow::bail!("unsupported platform: {}", s),
        }
    }
}

#[derive(Clone)]
pub struct MetaClient {
    inner: Client,
    creds: MetaCredentials,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReelPublishResponse {
    pub media_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub text: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub created_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentListResponse {
    pub data: Vec<Comment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyResponse {
    pub reply_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightValue {
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub name: String,
    pub period: String,
    pub values: Vec<InsightValue>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightsResponse {
    pub data: Vec<Insight>,
}

impl MetaClient {
    #[allow(dead_code)]
    pub fn new(creds: MetaCredentials) -> Self {
        Self::with_base(creds, DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base(creds: MetaCredentials, base_url: String) -> Self {
        Self {
            inner: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("build reqwest client"),
            creds,
            base_url,
        }
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        };
        self.inner.request(method, &url)
    }

    async fn send(&self, req: RequestBuilder) -> Result<Response> {
        let resp = req.send().await.context("HTTP request failed")?;
        Ok(resp)
    }

    pub async fn post_reel(
        &self,
        video_path: &str,
        caption: &str,
        platform: Platform,
        thumb: Option<&str>,
    ) -> Result<ReelPublishResponse> {
        match platform {
            Platform::Instagram => self.post_ig_reel(video_path, caption, thumb).await,
            Platform::Facebook => self.post_fb_reel(video_path, caption).await,
        }
    }

    async fn post_ig_reel(
        &self,
        video_path: &str,
        caption: &str,
        thumb: Option<&str>,
    ) -> Result<ReelPublishResponse> {
        let mut params: Vec<(&str, &str)> = vec![
            ("media_type", "REELS"),
            ("video_url", video_path),
            ("caption", caption),
        ];
        if let Some(t) = thumb {
            params.push(("cover_url", t));
        }
        params.push(("access_token", &self.creds.page_access_token));

        let create_path = format!("/{}/media", self.creds.ig_user_id);
        let resp = self
            .send(self.request(Method::POST, &create_path).form(&params))
            .await?;
        let container: IdResponse = check_status(resp).await?;

        let state = upload_state::MetaUploadState {
            video_url: video_path.to_string(),
            caption: caption.to_string(),
            platform: "instagram".to_string(),
            container_id: Some(container.id.clone()),
            media_id: None,
            step: "container_created".to_string(),
        };
        upload_state::save(&state).await.ok();

        let publish_params: Vec<(&str, &str)> = vec![
            ("creation_id", &container.id),
            ("access_token", &self.creds.page_access_token),
        ];
        let publish_path = format!("/{}/media_publish", self.creds.ig_user_id);
        let resp = self
            .send(self.request(Method::POST, &publish_path).form(&publish_params))
            .await?;
        let published: IdResponse = check_status(resp).await?;

        let mut state = state;
        state.media_id = Some(published.id.clone());
        state.step = "published".to_string();
        upload_state::save(&state).await.ok();
        upload_state::clear(video_path).await;

        Ok(ReelPublishResponse {
            media_id: published.id,
        })
    }

    async fn post_fb_reel(&self, video_path: &str, caption: &str) -> Result<ReelPublishResponse> {
        let params: Vec<(&str, &str)> = vec![
            ("file_url", video_path),
            ("description", caption),
            ("access_token", &self.creds.page_access_token),
        ];
        let path = format!("/{}/videos", self.creds.page_id);
        let state = upload_state::MetaUploadState {
            video_url: video_path.to_string(),
            caption: caption.to_string(),
            platform: "facebook".to_string(),
            container_id: None,
            media_id: None,
            step: "publishing".to_string(),
        };
        upload_state::save(&state).await.ok();
        let resp = self
            .send(self.request(Method::POST, &path).form(&params))
            .await?;
        let published: IdResponse = check_status(resp).await?;
        let mut state = state;
        state.media_id = Some(published.id.clone());
        state.step = "published".to_string();
        upload_state::save(&state).await.ok();
        upload_state::clear(video_path).await;
        Ok(ReelPublishResponse {
            media_id: published.id,
        })
    }

    pub async fn list_comments(
        &self,
        media_id: &str,
        platform: Platform,
    ) -> Result<Vec<Comment>> {
        let params: Vec<(&str, &str)> = vec![("access_token", &self.creds.page_access_token)];
        let path = match platform {
            Platform::Instagram => format!("/{}/comments", media_id),
            Platform::Facebook => format!("/{}/comments", media_id),
        };
        let resp = self
            .send(self.request(Method::GET, &path).query(&params))
            .await?;
        let parsed: CommentListResponse = check_status(resp).await?;
        Ok(parsed.data)
    }

    pub async fn reply(
        &self,
        comment_id: &str,
        platform: Platform,
        text: &str,
    ) -> Result<ReplyResponse> {
        let params: Vec<(&str, &str)> = vec![
            ("message", text),
            ("access_token", &self.creds.page_access_token),
        ];
        let path = match platform {
            Platform::Instagram => format!("/{}/replies", comment_id),
            Platform::Facebook => format!("/{}/comments", comment_id),
        };
        let resp = self
            .send(self.request(Method::POST, &path).form(&params))
            .await?;
        let parsed: IdResponse = check_status(resp).await?;
        Ok(ReplyResponse {
            reply_id: parsed.id,
        })
    }

    pub async fn get_insights(
        &self,
        media_id: &str,
        platform: Platform,
        metrics: &[String],
    ) -> Result<Vec<Insight>> {
        let metrics_str = metrics.join(",");
        let params: Vec<(&str, &str)> = vec![
            ("metric", &metrics_str),
            ("access_token", &self.creds.page_access_token),
        ];
        let path = match platform {
            Platform::Instagram => format!("/{}/insights", media_id),
            Platform::Facebook => format!("/{}/insights", media_id),
        };
        let resp = self
            .send(self.request(Method::GET, &path).query(&params))
            .await?;
        let parsed: InsightsResponse = check_status(resp).await?;
        Ok(parsed.data)
    }
}

async fn check_status<T: for<'de> Deserialize<'de>>(resp: Response) -> Result<T> {
    let status = resp.status();
    if status.is_success() {
        resp.json().await.context("decode JSON response")
    } else {
        let text = resp.text().await.unwrap_or_default();
        if status == StatusCode::TOO_MANY_REQUESTS {
            warn!("Meta Graph API rate limited");
        }
        Err(anyhow::anyhow!("Meta Graph API error {}: {}", status, text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_creds() -> MetaCredentials {
        MetaCredentials {
            app_id: "app-id".to_string(),
            app_secret: "app-secret".to_string(),
            page_access_token: "test-token".to_string(),
            ig_user_id: "ig-user-123".to_string(),
            page_id: "page-123".to_string(),
        }
    }

    #[tokio::test]
    async fn test_post_ig_reel() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/ig-user-123/media"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "container-1"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/ig-user-123/media_publish"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "media-1"
            })))
            .mount(&server)
            .await;

        let client = MetaClient::with_base(test_creds(), server.uri());
        let resp = client
            .post_reel(
                "https://example.com/video.mp4",
                "Test caption",
                Platform::Instagram,
                Some("https://example.com/thumb.jpg"),
            )
            .await
            .unwrap();
        assert_eq!(resp.media_id, "media-1");
    }

    #[tokio::test]
    async fn test_post_fb_reel() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/page-123/videos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video-1"
            })))
            .mount(&server)
            .await;

        let client = MetaClient::with_base(test_creds(), server.uri());
        let resp = client
            .post_reel(
                "https://example.com/video.mp4",
                "Test caption",
                Platform::Facebook,
                None,
            )
            .await
            .unwrap();
        assert_eq!(resp.media_id, "video-1");
    }

    #[tokio::test]
    async fn test_list_comments() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/media-1/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"id": "c1", "text": "Nice!", "username": "user1", "created_time": "2026-07-01T00:00:00+0000"},
                    {"id": "c2", "text": "Love it", "username": "user2"}
                ]
            })))
            .mount(&server)
            .await;

        let client = MetaClient::with_base(test_creds(), server.uri());
        let comments = client
            .list_comments("media-1", Platform::Instagram)
            .await
            .unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text.as_deref().unwrap(), "Nice!");
    }

    #[tokio::test]
    async fn test_reply_instagram() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/c1/replies"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "reply-1"
            })))
            .mount(&server)
            .await;

        let client = MetaClient::with_base(test_creds(), server.uri());
        let resp = client.reply("c1", Platform::Instagram, "Thanks!").await.unwrap();
        assert_eq!(resp.reply_id, "reply-1");
    }

    #[tokio::test]
    async fn test_reply_facebook() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/c1/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "reply-2"
            })))
            .mount(&server)
            .await;

        let client = MetaClient::with_base(test_creds(), server.uri());
        let resp = client.reply("c1", Platform::Facebook, "Thanks!").await.unwrap();
        assert_eq!(resp.reply_id, "reply-2");
    }

    #[tokio::test]
    async fn test_get_insights() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/media-1/insights"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {
                        "name": "plays",
                        "period": "lifetime",
                        "values": [{"value": 123}],
                        "title": "Plays",
                        "description": "Total video plays"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = MetaClient::with_base(test_creds(), server.uri());
        let insights = client
            .get_insights("media-1", Platform::Instagram, &["plays".to_string(), "likes".to_string()])
            .await
            .unwrap();
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].name, "plays");
    }
}
