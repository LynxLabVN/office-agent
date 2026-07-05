use crate::oauth::{self, Credentials};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use chrono_tz::America::Los_Angeles;
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::sync::Mutex;
use tracing::{debug, warn};

const DEFAULT_BASE_URL: &str = "https://www.googleapis.com";
const DEFAULT_DAILY_QUOTA: u64 = 10_000;
const UPLOAD_COST: u64 = 1_600;
const MOCK_VIDEO_ID: &str = "fake-video-id";
const DEFAULT_CHUNK_SIZE: usize = 1_048_576; // 1 MiB

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadState {
    video_path: String,
    session_url: String,
    file_len: u64,
    bytes_uploaded: u64,
    #[serde(default)]
    video_id: Option<String>,
}

#[derive(Clone)]
pub struct YouTubeClient {
    inner: Client,
    credentials: Credentials,
    access_token: Arc<Mutex<Option<String>>>,
    base_url: String,
    quota_path: PathBuf,
    mock_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct QuotaState {
    #[serde(default = "today_pt")]
    date: String,
    #[serde(default)]
    used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub text: String,
    pub published_at: String,
    #[serde(default)]
    pub like_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStats {
    pub view_count: String,
    pub like_count: String,
    pub comment_count: String,
}

impl YouTubeClient {
    #[allow(dead_code)]
    pub fn new(credentials: Credentials) -> Result<Self> {
        Self::with_base(credentials, DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base(credentials: Credentials, base_url: String) -> Result<Self> {
        let quota_path = default_quota_path()?;
        let mock_mode = base_url != DEFAULT_BASE_URL;
        Ok(Self {
            inner: Client::builder()
                .timeout(StdDuration::from_secs(120))
                .build()
                .context("build reqwest client")?,
            credentials,
            access_token: Arc::new(Mutex::new(None)),
            base_url,
            quota_path,
            mock_mode,
        })
    }

    #[allow(dead_code)]
    pub fn with_base_and_quota(
        credentials: Credentials,
        base_url: String,
        quota_path: impl AsRef<Path>,
    ) -> Self {
        let mock_mode = base_url != DEFAULT_BASE_URL;
        Self {
            inner: Client::builder()
                .timeout(StdDuration::from_secs(120))
                .build()
                .expect("build reqwest client"),
            credentials,
            access_token: Arc::new(Mutex::new(None)),
            base_url,
            quota_path: quota_path.as_ref().to_path_buf(),
            mock_mode,
        }
    }

    #[cfg(test)]
    pub fn set_mock_mode(&mut self, mock: bool) {
        self.mock_mode = mock;
    }

    fn data_url(&self, path: &str) -> String {
        format!("{}/youtube/v3{}", self.base_url.trim_end_matches('/'), path)
    }

    fn upload_url(&self) -> String {
        format!("{}/upload/youtube/v3/videos", self.base_url.trim_end_matches('/'))
    }

    fn analytics_url(&self, path: &str) -> String {
        format!("{}/v2{}", self.base_url.trim_end_matches('/'), path)
    }

    async fn get_token(&self) -> Result<String> {
        if let Some(token) = self.access_token.lock().await.clone() {
            return Ok(token);
        }
        let token = oauth::refresh_access_token(&self.credentials).await?;
        *self.access_token.lock().await = Some(token.clone());
        Ok(token)
    }

    async fn send_authenticated<F>(&self, build: F) -> Result<Response>
    where
        F: Fn(&str) -> RequestBuilder,
    {
        let token = self.get_token().await?;
        let resp = build(&token)
            .send()
            .await
            .context("HTTP request failed")?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            debug!("access token expired; refreshing");
            let new_token = oauth::refresh_access_token(&self.credentials).await?;
            *self.access_token.lock().await = Some(new_token.clone());
            let resp = build(&new_token)
                .send()
                .await
                .context("HTTP request failed")?;
            return Ok(resp);
        }
        Ok(resp)
    }

    pub async fn upload(
        &self,
        video_path: &str,
        title: &str,
        description: &str,
        tags: Vec<String>,
        category_id: u32,
        privacy: &str,
    ) -> Result<String> {
        self.check_upload_quota().await?;

        let token = self.get_token().await?;
        let file_len = if self.mock_mode {
            0u64
        } else {
            tokio::fs::metadata(video_path)
                .await
                .context("read video metadata")?
                .len()
        };

        let metadata = serde_json::json!({
            "snippet": {
                "title": title,
                "description": description,
                "tags": tags,
                "categoryId": category_id,
            },
            "status": {
                "privacyStatus": privacy,
            },
        });

        let initiate = |t: &str| -> RequestBuilder {
            self.inner
                .post(self.upload_url())
                .query(&[("uploadType", "resumable"), ("part", "snippet,status")])
                .bearer_auth(t)
                .header("Content-Type", "application/json; charset=UTF-8")
                .header("X-Upload-Content-Length", file_len)
                .header("X-Upload-Content-Type", "video/*")
                .json(&metadata)
        };

        let mut resp = initiate(&token).send().await.context("initiate upload failed")?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            let new_token = oauth::refresh_access_token(&self.credentials).await?;
            *self.access_token.lock().await = Some(new_token.clone());
            resp = initiate(&new_token)
                .send()
                .await
                .context("initiate upload failed")?;
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("upload initiate failed {}: {}", status, text);
        }

        if self.mock_mode {
            return Ok(MOCK_VIDEO_ID.to_string());
        }

        let location = resp
            .headers()
            .get("Location")
            .context("no upload location returned")?
            .to_str()
            .context("invalid location header")?
            .to_string();

        let state = UploadState {
            video_path: video_path.to_string(),
            session_url: location,
            file_len,
            bytes_uploaded: 0,
            video_id: None,
        };
        save_upload_state(&state).await?;

        self.upload_from_state(state).await
    }

    /// Resume an interrupted upload from saved state, continuing at the last
    /// uploaded byte.
    pub async fn resume_upload(&self, video_path: &str) -> Result<String> {
        let state = load_upload_state(video_path)
            .await
            .context("no resumable upload state found")?;
        if state.video_id.is_some() {
            return Ok(state.video_id.unwrap());
        }
        self.upload_from_state(state).await
    }

    async fn upload_from_state(&self, mut state: UploadState) -> Result<String> {
        let bytes = tokio::fs::read(&state.video_path)
            .await
            .context("read video file")?;
        let total = state.file_len as usize;
        let mut start = state.bytes_uploaded as usize;

        while start < total {
            let end = (start + DEFAULT_CHUNK_SIZE).min(total).saturating_sub(1);
            let chunk = &bytes[start..=end];
            let content_range = format!("bytes {}-{}/{}", start, end, total);

            let put_resp = self
                .inner
                .put(&state.session_url)
                .bearer_auth(self.get_token().await?)
                .header("Content-Type", "video/*")
                .header("Content-Range", &content_range)
                .body(chunk.to_vec())
                .send()
                .await
                .context("upload chunk request failed")?;

            let status = put_resp.status();
            if status == StatusCode::PERMANENT_REDIRECT || status == StatusCode::PARTIAL_CONTENT
            {
                // YouTube returns 308/ResumeIncomplete with Range header telling
                // us how many bytes it actually received.
                let range = put_resp
                    .headers()
                    .get("Range")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                let received = parse_range_end(range).unwrap_or(start as u64);
                state.bytes_uploaded = received;
                save_upload_state(&state).await?;
                start = received as usize;
                continue;
            }

            if !status.is_success() {
                let text = put_resp.text().await.unwrap_or_default();
                state.bytes_uploaded = start as u64;
                save_upload_state(&state).await?;
                bail!("upload chunk failed {}: {}", status, text);
            }

            // Success for this chunk; advance.
            start = end + 1;
            state.bytes_uploaded = start as u64;
            save_upload_state(&state).await?;

            // Final chunk returns 200/201 with the video resource.
            if start >= total {
                let video: Value = check_status(put_resp).await?;
                let video_id = video["id"]
                    .as_str()
                    .context("missing video id in upload response")?
                    .to_string();
                state.video_id = Some(video_id.clone());
                save_upload_state(&state).await?;
                clear_upload_state(&state.video_path).await;
                return Ok(video_id);
            }
        }

        Ok(state.video_id.context("upload completed but no video id returned")?)
    }

    pub async fn list_comments(&self, video_id: &str, max_results: u32) -> Result<Vec<Comment>> {
        let resp = self
            .send_authenticated(|token| {
                self.inner
                    .get(self.data_url("/commentThreads"))
                    .query(&[
                        ("part", "snippet"),
                        ("videoId", video_id),
                        ("maxResults", &max_results.to_string()),
                    ])
                    .bearer_auth(token)
            })
            .await?;
        let val: Value = check_status(resp).await?;
        Ok(parse_comments(&val))
    }

    pub async fn reply_comment(&self, comment_id: &str, text: &str) -> Result<String> {
        let body = serde_json::json!({
            "snippet": {
                "parentId": comment_id,
                "textOriginal": text,
            }
        });
        let resp = self
            .send_authenticated(|token| {
                self.inner
                    .post(self.data_url("/comments"))
                    .query(&[("part", "snippet")])
                    .bearer_auth(token)
                    .json(&body)
            })
            .await?;
        let val: Value = check_status(resp).await?;
        Ok(val["id"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    pub async fn get_stats(&self, video_id: &str) -> Result<VideoStats> {
        let resp = self
            .send_authenticated(|token| {
                self.inner
                    .get(self.data_url("/videos"))
                    .query(&[("part", "statistics"), ("id", video_id)])
                    .bearer_auth(token)
            })
            .await?;
        let val: Value = check_status(resp).await?;
        let stats = val["items"]
            .get(0)
            .and_then(|i| i.get("statistics"))
            .context("no statistics found for video")?;
        Ok(VideoStats {
            view_count: stats["viewCount"].as_str().unwrap_or("0").to_string(),
            like_count: stats["likeCount"].as_str().unwrap_or("0").to_string(),
            comment_count: stats["commentCount"].as_str().unwrap_or("0").to_string(),
        })
    }

    pub async fn get_video_analytics(
        &self,
        video_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Value> {
        let filters = format!("video=={}", video_id);
        let resp = self
            .send_authenticated(|token| {
                self.inner
                    .get(self.analytics_url("/reports"))
                    .query(&[
                        ("ids", "channel==MINE"),
                        ("startDate", start_date),
                        ("endDate", end_date),
                        (
                            "metrics",
                            "views,estimatedMinutesWatched,averageViewDuration,averageViewPercentage,subscribersGained",
                        ),
                        ("filters", &filters),
                    ])
                    .bearer_auth(token)
            })
            .await?;
        check_status(resp).await
    }

    async fn check_upload_quota(&self) -> Result<()> {
        let mut state = load_quota(&self.quota_path).await?;
        let remaining = DEFAULT_DAILY_QUOTA.saturating_sub(state.used);
        if remaining < UPLOAD_COST {
            let retry = next_midnight_pt();
            bail!(
                "YouTube upload quota exceeded. retry_after: {}",
                retry.to_rfc3339()
            );
        }
        state.used += UPLOAD_COST;
        save_quota(&self.quota_path, &state).await?;
        Ok(())
    }
}

async fn check_status<T: for<'de> Deserialize<'de>>(resp: Response) -> Result<T> {
    let status = resp.status();
    if status.is_success() {
        resp.json().await.context("decode JSON response")
    } else {
        let text = resp.text().await.unwrap_or_default();
        if status == StatusCode::TOO_MANY_REQUESTS {
            warn!("YouTube API rate limited");
        }
        Err(anyhow::anyhow!("YouTube API error {}: {}", status, text))
    }
}

fn parse_comments(val: &Value) -> Vec<Comment> {
    val["items"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| {
            let snippet = item.get("snippet")?;
            let top = snippet.get("topLevelComment")?.get("snippet")?;
            Some(Comment {
                id: item["id"].as_str()?.to_string(),
                author: top["authorDisplayName"].as_str()?.to_string(),
                text: top["textDisplay"].as_str()?.to_string(),
                published_at: top["publishedAt"].as_str()?.to_string(),
                like_count: top["likeCount"].as_str().unwrap_or("0").parse().unwrap_or(0),
            })
        })
        .collect()
}

fn default_quota_path() -> Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .context("HOME not set")?;
    Ok(PathBuf::from(home).join(".hermes/data/yt_quota.json"))
}

fn upload_state_dir() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hermes/data/uploads")
}

fn upload_state_path(video_path: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    video_path.hash(&mut hasher);
    let name = format!("upload_{:016x}.json", hasher.finish());
    upload_state_dir().join(name)
}

async fn save_upload_state(state: &UploadState) -> Result<()> {
    let path = upload_state_path(&state.video_path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("create upload state directory")?;
    }
    tokio::fs::write(&path, serde_json::to_vec_pretty(state)?)
        .await
        .context("write upload state")?;
    Ok(())
}

async fn load_upload_state(video_path: &str) -> Option<UploadState> {
    let path = upload_state_path(video_path);
    if !path.exists() {
        return None;
    }
    let bytes = tokio::fs::read(&path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

async fn clear_upload_state(video_path: &str) {
    let path = upload_state_path(video_path);
    let _ = tokio::fs::remove_file(&path).await;
}

fn today_pt() -> String {
    Los_Angeles
        .from_utc_datetime(&Utc::now().naive_utc())
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

fn parse_range_end(range: &str) -> Option<u64> {
    // Range: bytes=0-12345 means bytes 0..=12345 were received, i.e. 12346 bytes.
    let parts: Vec<&str> = range.split('=').collect();
    if parts.len() != 2 {
        return None;
    }
    let nums: Vec<&str> = parts[1].split('-').collect();
    if nums.len() != 2 {
        return None;
    }
    nums[1].parse::<u64>().ok().map(|n| n + 1)
}

fn next_midnight_pt() -> DateTime<chrono_tz::Tz> {
    let now = Los_Angeles.from_utc_datetime(&Utc::now().naive_utc());
    let today_midnight = Los_Angeles
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .expect("valid midnight");
    if now <= today_midnight {
        today_midnight
    } else {
        today_midnight + Duration::days(1)
    }
}

async fn load_quota(path: &Path) -> Result<QuotaState> {
    if !path.exists() {
        return Ok(QuotaState {
            date: today_pt(),
            used: 0,
        });
    }
    let bytes = tokio::fs::read(path)
        .await
        .context("read quota file")?;
    let mut state: QuotaState = serde_json::from_slice(&bytes).unwrap_or_default();
    let today = today_pt();
    if state.date != today {
        state.date = today;
        state.used = 0;
    }
    Ok(state)
}

async fn save_quota(path: &Path, state: &QuotaState) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("create quota directory")?;
    }
    tokio::fs::write(path, serde_json::to_vec_pretty(state)?)
        .await
        .context("write quota file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn creds() -> Credentials {
        Credentials {
            client_id: "client-id".to_string(),
            client_secret: "client-secret".to_string(),
            refresh_token: "refresh-token".to_string(),
        }
    }

    fn test_client(server_uri: &str, quota: &Path) -> YouTubeClient {
        let mut client = YouTubeClient::with_base_and_quota(creds(), server_uri.to_string(), quota);
        client.access_token = Arc::new(Mutex::new(Some("test-token".to_string())));
        client
    }

    fn quota_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("yt_quota_{}.json", name))
    }

    fn write_quota(path: &Path, used: u64) {
        let state = QuotaState {
            date: today_pt(),
            used,
        };
        std::fs::create_dir_all(path.parent().unwrap()).ok();
        std::fs::write(path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    }

    #[tokio::test]
    async fn test_list_comments() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/youtube/v3/commentThreads"))
            .and(query_param("part", "snippet"))
            .and(query_param("videoId", "VIDEO123"))
            .and(query_param("maxResults", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [{
                    "id": "c1",
                    "snippet": {
                        "topLevelComment": {
                            "snippet": {
                                "authorDisplayName": "Alice",
                                "textDisplay": "Great video!",
                                "publishedAt": "2026-07-01T12:00:00Z",
                                "likeCount": "5"
                            }
                        }
                    }
                }]
            })))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), &quota_path("list_comments"));
        let comments = client.list_comments("VIDEO123", 10).await.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].id, "c1");
        assert_eq!(comments[0].author, "Alice");
    }

    #[tokio::test]
    async fn test_reply_comment() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/youtube/v3/comments"))
            .and(query_param("part", "snippet"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "reply-id-1"
            })))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), &quota_path("reply_comment"));
        let id = client.reply_comment("c1", "Thanks!").await.unwrap();
        assert_eq!(id, "reply-id-1");
    }

    #[tokio::test]
    async fn test_get_stats() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/youtube/v3/videos"))
            .and(query_param("part", "statistics"))
            .and(query_param("id", "VIDEO123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [{
                    "statistics": {
                        "viewCount": "1234",
                        "likeCount": "56",
                        "commentCount": "7"
                    }
                }]
            })))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), &quota_path("get_stats"));
        let stats = client.get_stats("VIDEO123").await.unwrap();
        assert_eq!(stats.view_count, "1234");
        assert_eq!(stats.like_count, "56");
        assert_eq!(stats.comment_count, "7");
    }

    #[tokio::test]
    async fn test_get_video_analytics() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/reports"))
            .and(query_param("ids", "channel==MINE"))
            .and(query_param("startDate", "2026-06-01"))
            .and(query_param("endDate", "2026-06-30"))
            .and(query_param("filters", "video==VIDEO123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "kind": "youtubeAnalytics#resultTable",
                "rows": [["2026-06-01", 100, 200, 30, 0.5, 1]]
            })))
            .mount(&server)
            .await;

        let client = test_client(&server.uri(), &quota_path("analytics"));
        let data = client
            .get_video_analytics("VIDEO123", "2026-06-01", "2026-06-30")
            .await
            .unwrap();
        assert_eq!(data["kind"], "youtubeAnalytics#resultTable");
    }

    #[tokio::test]
    async fn test_upload_mock() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/upload/youtube/v3/videos"))
            .and(query_param("uploadType", "resumable"))
            .and(query_param("part", "snippet,status"))
            .and(header("X-Upload-Content-Length", "0"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Location", "http://localhost/upload/session"),
            )
            .mount(&server)
            .await;

        let q = quota_path("upload_mock");
        write_quota(&q, 0);
        let client = test_client(&server.uri(), &q);
        let id = client
            .upload("/dev/null", "Title", "Desc", vec!["tag".into()], 22, "private")
            .await
            .unwrap();
        assert_eq!(id, MOCK_VIDEO_ID);
    }

    #[tokio::test]
    async fn test_resumable_upload_recovers_from_50_percent() {
        let server = MockServer::start().await;
        let session_url = format!("{}/upload/session", server.uri());
        let total = 1_048_576usize; // 1 MiB
        let half_end = total / 2 - 1; // Range end at 50%

        // Create a temp file with deterministic bytes.
        let tmp_dir = std::env::temp_dir().join("yt_resumable_test");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let video_path = tmp_dir.join("video.bin");
        std::fs::write(&video_path, vec![0xABu8; total]).unwrap();

        Mock::given(method("POST"))
            .and(path("/upload/youtube/v3/videos"))
            .and(query_param("uploadType", "resumable"))
            .and(query_param("part", "snippet,status"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("Location", session_url.clone()),
            )
            .mount(&server)
            .await;

        // First full-file chunk: server acknowledges only 50%.
        Mock::given(method("PUT"))
            .and(path("/upload/session"))
            .and(header(
                "Content-Range",
                format!("bytes 0-{}/{}", total - 1, total).as_str(),
            ))
            .respond_with(
                ResponseTemplate::new(308)
                    .insert_header("Range", format!("bytes=0-{}", half_end)),
            )
            .mount(&server)
            .await;

        // Second chunk (from 50% onward) fails with a 500.
        Mock::given(method("PUT"))
            .and(path("/upload/session"))
            .and(header(
                "Content-Range",
                format!("bytes {}-{}/{}", half_end + 1, total - 1, total).as_str(),
            ))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&server)
            .await;

        let q = quota_path("resumable");
        write_quota(&q, 0);
        let mut client = test_client(&server.uri(), &q);
        client.set_mock_mode(false);

        // Upload fails partway through.
        let err = client
            .upload(
                video_path.to_str().unwrap(),
                "Title",
                "Desc",
                vec![],
                22,
                "private",
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("chunk failed"));

        // State should record bytes_uploaded at 50%.
        let state = load_upload_state(video_path.to_str().unwrap())
            .await
            .expect("upload state saved");
        assert_eq!(state.bytes_uploaded, half_end as u64 + 1);

        // Now mock only the resume chunk returning success.
        let _ = server.reset().await;
        Mock::given(method("PUT"))
            .and(path("/upload/session"))
            .and(header(
                "Content-Range",
                format!("bytes {}-{}/{}", half_end + 1, total - 1, total).as_str(),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resumed-video-id"
            })))
            .mount(&server)
            .await;

        let id = client.resume_upload(video_path.to_str().unwrap()).await.unwrap();
        assert_eq!(id, "resumed-video-id");

        // Cleanup.
        let _ = std::fs::remove_file(&video_path);
        clear_upload_state(video_path.to_str().unwrap()).await;
    }

    #[tokio::test]
    async fn test_quota_decrement() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/upload/youtube/v3/videos"))
            .and(query_param("uploadType", "resumable"))
            .and(query_param("part", "snippet,status"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let q = quota_path("quota_decrement");
        write_quota(&q, 0);
        let client = test_client(&server.uri(), &q);
        client
            .upload("/dev/null", "Title", "Desc", vec![], 22, "private")
            .await
            .unwrap();

        let bytes = tokio::fs::read(&q).await.unwrap();
        let state: QuotaState = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(state.used, UPLOAD_COST);
    }

    #[tokio::test]
    async fn test_quota_exceeded() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/upload/youtube/v3/videos"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let q = quota_path("quota_exceeded");
        write_quota(&q, DEFAULT_DAILY_QUOTA - UPLOAD_COST + 1);
        let client = test_client(&server.uri(), &q);
        let err = client
            .upload("/dev/null", "Title", "Desc", vec![], 22, "private")
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("quota exceeded"), "{}", msg);
        assert!(msg.contains("retry_after"), "{}", msg);
    }
}
