use std::path::Path;

use anyhow::{Context, Result};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::{debug, warn};

use crate::oauth::TikTokOAuth;

pub const DEFAULT_BASE_URL: &str = "https://open.tiktokapis.com/v2";

#[derive(Clone)]
pub struct TikTokClient {
    inner: Client,
    auth: TikTokOAuth,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostedVideo {
    pub video_id: String,
    pub share_url: String,
    pub publish_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetrics {
    pub video_id: String,
    pub view_count: u64,
    pub like_count: u64,
    pub comment_count: u64,
    pub share_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub comment_id: String,
    pub text: String,
    pub username: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ApiError {
    code: String,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ApiResponse<T> {
    data: T,
    error: ApiError,
}

#[derive(Debug, Clone, Deserialize)]
struct InitUploadData {
    upload_url: String,
    publish_id: String,
}

impl TikTokClient {
    #[allow(dead_code)]
    pub fn new(auth: TikTokOAuth) -> Self {
        Self::with_base(auth, DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base(auth: TikTokOAuth, base_url: String) -> Self {
        Self {
            inner: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("build reqwest client"),
            auth,
            base_url,
        }
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        };
        self.inner
            .request(method, &url)
            .header("Authorization", self.auth.auth_header())
    }

    async fn send(&self, req: RequestBuilder) -> Result<Response> {
        debug!("TikTok API request");
        let resp = req.send().await.context("HTTP request failed")?;
        Ok(resp)
    }

    async fn parse<T: DeserializeOwned>(resp: Response) -> Result<T> {
        let status = resp.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            warn!("TikTok API rate limited");
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("TikTok API error {}: {}", status, text);
        }
        let wrapper: ApiResponse<T> = resp.json().await.context("decode JSON response")?;
        if wrapper.error.code != "ok" {
            anyhow::bail!(
                "TikTok API returned error {}: {}",
                wrapper.error.code,
                wrapper.error.message.unwrap_or_default()
            );
        }
        Ok(wrapper.data)
    }

    pub async fn post_video(
        &self,
        video_path: &Path,
        title: &str,
        privacy_level: &str,
        tags: &[String],
    ) -> Result<PostedVideo> {
        let video_bytes = tokio::fs::read(video_path)
            .await
            .with_context(|| format!("read video file {}", video_path.display()))?;
        let video_size = video_bytes.len() as u64;
        anyhow::ensure!(video_size > 0, "video file is empty");

        let init_body = serde_json::json!({
            "post_info": {
                "title": title,
                "privacy_level": privacy_level,
                "tags": tags,
            },
            "source_info": {
                "source": "FILE_UPLOAD",
                "video_size": video_size,
                "chunk_size": video_size,
                "total_chunk_count": 1,
            }
        });

        let init: InitUploadData = Self::parse(
            self.send(
                self.request(Method::POST, "/post/publish/video/init/")
                    .json(&init_body),
            )
            .await?,
        )
        .await?;

        self.upload_chunk(&init.upload_url, &video_bytes).await?;

        let status_body = serde_json::json!({
            "publish_id": init.publish_id,
        });
        let status: PostedVideo = Self::parse(
            self.send(
                self.request(Method::POST, "/post/publish/status/fetch/")
                    .json(&status_body),
            )
            .await?,
        )
        .await?;

        Ok(status)
    }

    async fn upload_chunk(&self, upload_url: &str, bytes: &[u8]) -> Result<()> {
        let size = bytes.len();
        let content_range = format!("bytes 0-{}/{}", size.saturating_sub(1), size);
        let resp = self
            .inner
            .put(upload_url)
            .header("Content-Range", content_range)
            .header("Content-Type", "video/mp4")
            .body(bytes.to_vec())
            .send()
            .await
            .context("chunk upload request failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("TikTok chunk upload failed {}: {}", status, text);
        }
        Ok(())
    }

    pub async fn get_metrics(&self, video_id: &str) -> Result<VideoMetrics> {
        let body = serde_json::json!({
            "video_id": video_id,
        });
        Self::parse(
            self.send(self.request(Method::POST, "/video/query/").json(&body))
                .await?,
        )
        .await
    }

    pub async fn list_comments(
        &self,
        video_id: &str,
        max_results: Option<u32>,
    ) -> Result<Vec<Comment>> {
        let max_results = max_results.unwrap_or(20).min(100);
        #[derive(Deserialize)]
        struct CommentListData {
            comments: Vec<Comment>,
        }
        let data: CommentListData = Self::parse(
            self.send(
                self.request(Method::GET, "/comment/list/").query(&[
                    ("video_id", video_id),
                    ("max_results", &max_results.to_string()),
                ]),
            )
            .await?,
        )
        .await?;
        Ok(data.comments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_auth() -> TikTokOAuth {
        TikTokOAuth {
            client_key: "test-key".to_string(),
            client_secret: "test-secret".to_string(),
            access_token: "test-token".to_string(),
        }
    }

    fn wrap<T: Serialize>(data: T) -> serde_json::Value {
        serde_json::json!({
            "data": data,
            "error": {"code": "ok", "message": ""},
        })
    }

    #[tokio::test]
    async fn test_post_video() {
        let server = MockServer::start().await;
        let base_url = server.uri();

        let video_dir = std::env::temp_dir().join("mcp-social-tiktok-test");
        std::fs::create_dir_all(&video_dir).ok();
        let video_path = video_dir.join("test.mp4");
        let mut file = std::fs::File::create(&video_path).unwrap();
        file.write_all(b"fake-video-bytes").unwrap();
        drop(file);

        let upload_url = format!("{}/upload", base_url.trim_end_matches('/'));

        Mock::given(method("POST"))
            .and(path("/post/publish/video/init/"))
            .and(body_json(serde_json::json!({
                "post_info": {
                    "title": "My TikTok",
                    "privacy_level": "PUBLIC_TO_EVERYONE",
                    "tags": ["fun", "test"],
                },
                "source_info": {
                    "source": "FILE_UPLOAD",
                    "video_size": 16,
                    "chunk_size": 16,
                    "total_chunk_count": 1,
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrap(serde_json::json!({
                "upload_url": upload_url,
                "publish_id": "pub-123",
            }))))
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/upload"))
            .and(header("content-range", "bytes 0-15/16"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/post/publish/status/fetch/"))
            .and(body_json(serde_json::json!({"publish_id": "pub-123"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrap(serde_json::json!({
                "video_id": "v-123",
                "share_url": "https://tiktok.com/@user/video/v-123",
                "publish_id": "pub-123",
            }))))
            .mount(&server)
            .await;

        let client = TikTokClient::with_base(test_auth(), base_url);
        let result = client
            .post_video(
                &video_path,
                "My TikTok",
                "PUBLIC_TO_EVERYONE",
                &["fun".to_string(), "test".to_string()],
            )
            .await
            .unwrap();
        assert_eq!(result.video_id, "v-123");
        assert!(result.share_url.contains("v-123"));
        assert_eq!(result.publish_id, "pub-123");
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/video/query/"))
            .and(body_json(serde_json::json!({"video_id": "v-456"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrap(serde_json::json!({
                "video_id": "v-456",
                "view_count": 1000,
                "like_count": 50,
                "comment_count": 10,
                "share_count": 5,
            }))))
            .mount(&server)
            .await;

        let client = TikTokClient::with_base(test_auth(), server.uri());
        let metrics = client.get_metrics("v-456").await.unwrap();
        assert_eq!(metrics.video_id, "v-456");
        assert_eq!(metrics.view_count, 1000);
        assert_eq!(metrics.like_count, 50);
        assert_eq!(metrics.comment_count, 10);
        assert_eq!(metrics.share_count, 5);
    }

    #[tokio::test]
    async fn test_list_comments() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/comment/list/"))
            .and(query_param("video_id", "v-789"))
            .and(query_param("max_results", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrap(serde_json::json!({
                "comments": [
                    {
                        "comment_id": "c-1",
                        "text": "Great video!",
                        "username": "alice",
                        "timestamp": "2026-07-05T00:00:00Z",
                    }
                ]
            }))))
            .mount(&server)
            .await;

        let client = TikTokClient::with_base(test_auth(), server.uri());
        let comments = client.list_comments("v-789", Some(10)).await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].comment_id, "c-1");
        assert_eq!(comments[0].username, "alice");
    }
}
