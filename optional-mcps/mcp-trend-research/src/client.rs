use anyhow::{bail, Context, Result};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tracing::warn;

#[allow(dead_code)]
const DEFAULT_TT_BASE_URL: &str = "https://research.tiktok.com";
const DEFAULT_YT_BASE_URL: &str = "https://www.googleapis.com";

#[derive(Clone)]
pub struct TrendClient {
    inner: Client,
    tt_client_key: Option<String>,
    tt_client_secret: Option<String>,
    yt_api_key: Option<String>,
    tt_base_url: String,
    yt_base_url: String,
    tt_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub hook_text: String,
    pub platform: String,
    pub views: u64,
    pub likes: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingAudio {
    pub audio_id: String,
    pub title: String,
    pub platform: String,
    pub uses_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceVideo {
    pub video_id: String,
    pub url: String,
    pub title: String,
    pub views: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokTokenResponse {
    pub access_token: String,
}

impl TrendClient {
    #[allow(dead_code)]
    pub fn new() -> Result<Self> {
        Self::with_base(
            DEFAULT_TT_BASE_URL.to_string(),
            DEFAULT_YT_BASE_URL.to_string(),
        )
    }

    pub fn with_base(tt_base_url: String, yt_base_url: String) -> Result<Self> {
        let tt_client_key = env::var("TT_RESEARCH_CLIENT_KEY").ok();
        let tt_client_secret = env::var("TT_RESEARCH_CLIENT_SECRET").ok();
        let yt_api_key = env::var("YOUTUBE_API_KEY")
            .or_else(|_| env::var("YOUTUBE_CLIENT_ID"))
            .ok();

        // Gate: if key is empty or the explicit pending flag is set, treat TikTok
        // Research API as not yet approved.
        let tt_pending = tt_client_key.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
            || env::var("TT_RESEARCH_PENDING").is_ok();

        Ok(Self {
            inner: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .context("build reqwest client")?,
            tt_client_key,
            tt_client_secret,
            yt_api_key,
            tt_base_url,
            yt_base_url,
            tt_pending,
        })
    }

    #[allow(dead_code)]
    pub fn tiktok_pending(&self) -> bool {
        self.tt_pending
    }

    #[cfg(test)]
    pub fn test_client(tt_base_url: String, yt_base_url: String) -> Self {
        Self {
            inner: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            tt_client_key: Some("key".to_string()),
            tt_client_secret: Some("secret".to_string()),
            yt_api_key: Some("yt-key".to_string()),
            tt_base_url,
            yt_base_url,
            tt_pending: false,
        }
    }

    fn tt_url(&self, path: &str) -> String {
        format!("{}{}", self.tt_base_url.trim_end_matches('/'), path)
    }

    fn yt_url(&self, path: &str) -> String {
        format!("{}{}", self.yt_base_url.trim_end_matches('/'), path)
    }

    async fn tt_token(&self) -> Result<String> {
        let key = self
            .tt_client_key
            .as_ref()
            .context("TT_RESEARCH_CLIENT_KEY not set")?;
        let secret = self
            .tt_client_secret
            .as_ref()
            .context("TT_RESEARCH_CLIENT_SECRET not set")?;

        let resp = self
            .inner
            .post(self.tt_url("/v1/research/oauth/token/"))
            .form(&[
                ("client_key", key.as_str()),
                ("client_secret", secret.as_str()),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await
            .context("TT Research token request failed")?;

        let token: TikTokTokenResponse = check_status(resp).await?;
        Ok(token.access_token)
    }

    async fn tt_request(&self, method: reqwest::Method, path: &str) -> Result<RequestBuilder> {
        if self.tt_pending {
            bail!("TT Research API pending — see LEADTIMES.md");
        }
        let token = self.tt_token().await?;
        Ok(self
            .inner
            .request(method, self.tt_url(path))
            .bearer_auth(token))
    }

    pub async fn search_hooks(&self, product_category: &str, region: Option<&str>) -> Result<Vec<Hook>> {
        if self.tt_pending {
            bail!("TT Research API pending — see LEADTIMES.md");
        }

        let region = region.unwrap_or("VN");
        let req = self
            .tt_request(
                reqwest::Method::POST,
                "/v1/research/video/query/",
            )
            .await?
            .json(&serde_json::json!({
                "query": product_category,
                "region": region,
                "period": "7",
                "max_count": 10,
            }));

        let resp = req.send().await.context("TT Research search request failed")?;
        let val: serde_json::Value = check_status(resp).await?;
        Ok(parse_tt_hooks(&val))
    }

    pub async fn get_trending_audio(&self, region: Option<&str>) -> Result<Vec<TrendingAudio>> {
        if self.tt_pending {
            bail!("TT Research API pending — see LEADTIMES.md");
        }

        let region = region.unwrap_or("VN");
        let req = self
            .tt_request(
                reqwest::Method::POST,
                "/v1/research/tiktok_sounds/",
            )
            .await?
            .json(&serde_json::json!({
                "region": region,
                "max_count": 10,
            }));

        let resp = req.send().await.context("TT Research trending audio request failed")?;
        let val: serde_json::Value = check_status(resp).await?;
        Ok(parse_tt_audio(&val))
    }

    pub async fn fetch_reference_videos(
        &self,
        query: &str,
        platform: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ReferenceVideo>> {
        let limit = limit.unwrap_or(10).min(50);
        match platform.to_lowercase().as_str() {
            "youtube" => self.fetch_youtube_videos(query, limit).await,
            "tiktok" => {
                if self.tt_pending {
                    bail!("TT Research API pending — see LEADTIMES.md");
                }
                let req = self
                    .tt_request(reqwest::Method::POST, "/v1/research/video/query/")
                    .await?
                    .json(&serde_json::json!({
                        "query": query,
                        "period": "30",
                        "max_count": limit,
                    }));
                let resp = req.send().await.context("TT Research video query failed")?;
                let val: serde_json::Value = check_status(resp).await?;
                Ok(parse_tt_videos(&val))
            }
            _ => bail!("unsupported platform: {}. Use youtube or tiktok", platform),
        }
    }

    async fn fetch_youtube_videos(&self, query: &str, limit: u32) -> Result<Vec<ReferenceVideo>> {
        let key = self
            .yt_api_key
            .as_ref()
            .context("YOUTUBE_API_KEY not set")?;
        let resp = self
            .inner
            .get(self.yt_url("/youtube/v3/search"))
            .query(&[
                ("part", "snippet"),
                ("q", query),
                ("type", "video"),
                ("maxResults", &limit.to_string()),
                ("key", key),
            ])
            .send()
            .await
            .context("YouTube search request failed")?;

        let val: serde_json::Value = check_status(resp).await?;
        Ok(parse_yt_videos(&val))
    }
}

async fn check_status<T: for<'de> Deserialize<'de>>(resp: Response) -> Result<T> {
    let status = resp.status();
    if status.is_success() {
        resp.json().await.context("decode JSON response")
    } else {
        let text = resp.text().await.unwrap_or_default();
        if status == StatusCode::TOO_MANY_REQUESTS {
            warn!("external API rate limited");
        }
        Err(anyhow::anyhow!("API error {}: {}", status, text))
    }
}

fn parse_tt_hooks(val: &serde_json::Value) -> Vec<Hook> {
    val.get("data")
        .and_then(|d| d.get("videos"))
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| {
            Some(Hook {
                hook_text: item["video_description"]
                    .as_str()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                platform: "tiktok".to_string(),
                views: item["view_count"].as_str().unwrap_or("0").parse().unwrap_or(0),
                likes: item["like_count"].as_str().unwrap_or("0").parse().unwrap_or(0),
                created_at: item["create_time"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect()
}

fn parse_tt_audio(val: &serde_json::Value) -> Vec<TrendingAudio> {
    val.get("data")
        .and_then(|d| d.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| {
            Some(TrendingAudio {
                audio_id: item["music_id"].as_str()?.to_string(),
                title: item["title"].as_str().unwrap_or("").to_string(),
                platform: "tiktok".to_string(),
                uses_count: item["video_count"].as_str().unwrap_or("0").parse().unwrap_or(0),
            })
        })
        .collect()
}

fn parse_tt_videos(val: &serde_json::Value) -> Vec<ReferenceVideo> {
    val.get("data")
        .and_then(|d| d.get("videos"))
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| {
            let id = item["id"].as_str().or_else(|| item["video_id"].as_str())?;
            Some(ReferenceVideo {
                video_id: id.to_string(),
                url: format!("https://www.tiktok.com/@user/video/{}", id),
                title: item["video_description"]
                    .as_str()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                views: item["view_count"].as_str().unwrap_or("0").parse().unwrap_or(0),
            })
        })
        .collect()
}

fn parse_yt_videos(val: &serde_json::Value) -> Vec<ReferenceVideo> {
    val["items"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| {
            let id = item["id"]["videoId"].as_str()?;
            Some(ReferenceVideo {
                video_id: id.to_string(),
                url: format!("https://www.youtube.com/watch?v={}", id),
                title: item["snippet"]["title"].as_str().unwrap_or("").to_string(),
                views: 0,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_client(tt_uri: &str, yt_uri: &str) -> TrendClient {
        TrendClient::test_client(tt_uri.to_string(), yt_uri.to_string())
    }

    #[tokio::test]
    async fn test_search_hooks() {
        let tt_server = MockServer::start().await;
        let yt_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/research/oauth/token/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tt-token"
            })))
            .mount(&tt_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/research/video/query/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "videos": [{
                        "video_description": "First line of desc",
                        "view_count": "12345",
                        "like_count": "678",
                        "create_time": "2026-07-01T00:00:00Z"
                    }]
                }
            })))
            .mount(&tt_server)
            .await;

        let client = test_client(&tt_server.uri(), &yt_server.uri());
        let hooks = client.search_hooks("audio", Some("VN")).await.unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].platform, "tiktok");
        assert_eq!(hooks[0].views, 12345);
    }

    #[tokio::test]
    async fn test_get_trending_audio() {
        let tt_server = MockServer::start().await;
        let yt_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/research/oauth/token/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tt-token"
            })))
            .mount(&tt_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/research/tiktok_sounds/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "music_id": "snd-1",
                    "title": "Popular sound",
                    "video_count": "99999"
                }]
            })))
            .mount(&tt_server)
            .await;

        let client = test_client(&tt_server.uri(), &yt_server.uri());
        let audio = client.get_trending_audio(Some("VN")).await.unwrap();
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].audio_id, "snd-1");
    }

    #[tokio::test]
    async fn test_fetch_reference_videos_youtube() {
        let tt_server = MockServer::start().await;
        let yt_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/youtube/v3/search"))
            .and(query_param("part", "snippet"))
            .and(query_param("q", "vietnamese tech review"))
            .and(query_param("type", "video"))
            .and(query_param("maxResults", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [{
                    "id": { "videoId": "abc123" },
                    "snippet": { "title": "VN Tech Review" }
                }]
            })))
            .mount(&yt_server)
            .await;

        let client = test_client(&tt_server.uri(), &yt_server.uri());
        let videos = client
            .fetch_reference_videos("vietnamese tech review", "youtube", Some(5))
            .await
            .unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].video_id, "abc123");
        assert!(videos[0].url.contains("youtube.com"));
    }

    #[tokio::test]
    async fn test_tiktok_pending() {
        let mut client = test_client("http://localhost", "http://localhost");
        client.tt_pending = true;

        let err = client
            .search_hooks("audio", None)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("TT Research API pending"), "{}", err);
    }
}
