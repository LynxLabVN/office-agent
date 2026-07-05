use anyhow::{Context, Result};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::warn;

#[allow(dead_code)]
const DEFAULT_BASE_URL: &str = "https://openapi.zalo.me/v2.0/officialaccount";

#[derive(Clone)]
pub struct ZaloOaClient {
    inner: Client,
    access_token: String,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowersResponse {
    pub total: u64,
    pub count: u64,
    pub offset: u64,
    #[serde(default)]
    pub followers: Vec<Follower>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Follower {
    pub user_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

impl ZaloOaClient {
    #[allow(dead_code)]
    pub fn new(access_token: String) -> Self {
        Self::with_base(access_token, DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base(access_token: String, base_url: String) -> Self {
        Self {
            inner: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("build reqwest client"),
            access_token,
            base_url,
        }
    }

    fn url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.inner
            .request(method, self.url(path))
            .query(&[("access_token", self.access_token.as_str())])
    }

    async fn send(&self, req: RequestBuilder) -> Result<Response> {
        let resp = req.send().await.context("HTTP request failed")?;
        Ok(resp)
    }

    pub async fn send_oa_message(&self, user_id: &str, text: &str) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "recipient": { "user_id": user_id },
            "message": { "text": text }
        });
        let resp = self
            .send(self.request(Method::POST, "/message").json(&body))
            .await?;
        check_status(resp).await
    }

    pub async fn send_oa_message_template(
        &self,
        user_id: &str,
        template_id: &str,
        template_data: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "recipient": { "user_id": user_id },
            "message": {
                "template_id": template_id,
                "template_data": template_data
            }
        });
        let resp = self
            .send(self.request(Method::POST, "/message").json(&body))
            .await?;
        check_status(resp).await
    }

    pub async fn send_oa_attachment(
        &self,
        user_id: &str,
        attachment_type: &str,
        url: &str,
        caption: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut payload = serde_json::json!({ "url": url });
        if let Some(c) = caption {
            payload["caption"] = serde_json::Value::String(c.to_string());
        }
        let body = serde_json::json!({
            "recipient": { "user_id": user_id },
            "message": {
                "attachment": {
                    "type": attachment_type,
                    "payload": payload
                }
            }
        });
        let resp = self
            .send(self.request(Method::POST, "/message").json(&body))
            .await?;
        check_status(resp).await
    }

    pub async fn list_followers(&self, offset: u32, count: u32) -> Result<FollowersResponse> {
        let resp = self
            .send(
                self.request(Method::GET, "/getfollowers")
                    .query(&[("offset", offset.to_string()), ("count", count.to_string())]),
            )
            .await?;
        check_status(resp).await
    }

    pub async fn get_user_profile(&self, user_id: &str) -> Result<serde_json::Value> {
        crate::audit::log_pii_access("agent", user_id, "zalo_user_profile");
        let resp = self
            .send(
                self.request(Method::GET, "/getprofile")
                    .query(&[("user_id", user_id)]),
            )
            .await?;
        check_status(resp).await
    }

    pub async fn broadcast(
        &self,
        message: serde_json::Value,
        segment: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({ "message": message });
        if let Some(s) = segment {
            body["segment"] = s;
        }
        let resp = self
            .send(self.request(Method::POST, "/broadcast").json(&body))
            .await?;
        check_status(resp).await
    }

    pub async fn query_message(&self, message_id: &str) -> Result<serde_json::Value> {
        let resp = self
            .send(
                self.request(Method::GET, "/getmessagestatus")
                    .query(&[("msg_id", message_id)]),
            )
            .await?;
        check_status(resp).await
    }

    pub async fn tag_user(&self, user_id: &str, tag: &str) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "user_id": user_id,
            "tag_name": tag
        });
        let resp = self
            .send(self.request(Method::POST, "/tag/user").json(&body))
            .await?;
        check_status(resp).await
    }

    pub async fn get_oa_profile(&self) -> Result<serde_json::Value> {
        let resp = self.send(self.request(Method::GET, "/getoa")).await?;
        check_status(resp).await
    }

    pub async fn set_webhook(&self, url: &str) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "webhook": { "url": url, "subscribe_events": ["user_send_text", "user_send_image", "follow", "unfollow"] } });
        let resp = self
            .send(self.request(Method::POST, "/setwebhook").json(&body))
            .await?;
        check_status(resp).await
    }
}

async fn check_status<T: for<'de> Deserialize<'de>>(resp: Response) -> Result<T> {
    let status = resp.status();
    if status.is_success() {
        resp.json().await.context("decode JSON response")
    } else {
        let text = resp.text().await.unwrap_or_default();
        if status == StatusCode::TOO_MANY_REQUESTS {
            warn!("Zalo OA rate limited");
        }
        Err(anyhow::anyhow!("Zalo OA API error {}: {}", status, text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_client(base_url: String) -> ZaloOaClient {
        ZaloOaClient::with_base("test-token".to_string(), base_url)
    }

    #[tokio::test]
    async fn test_send_oa_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/message"))
            .and(query_param("access_token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message_id": "msg-123",
                "error": 0
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client.send_oa_message("user-1", "hello").await.unwrap();
        assert_eq!(resp["message_id"], "msg-123");
    }

    #[tokio::test]
    async fn test_send_oa_attachment() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/message"))
            .and(query_param("access_token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message_id": "msg-456",
                "error": 0
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client
            .send_oa_attachment("user-1", "image", "https://example.com/img.jpg", Some("caption"))
            .await
            .unwrap();
        assert_eq!(resp["message_id"], "msg-456");
    }

    #[tokio::test]
    async fn test_list_followers() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/getfollowers"))
            .and(query_param("access_token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total": 100,
                "count": 1,
                "offset": 0,
                "followers": [{ "user_id": "u1", "display_name": "Alice" }]
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client.list_followers(0, 50).await.unwrap();
        assert_eq!(resp.total, 100);
        assert_eq!(resp.followers.len(), 1);
        assert_eq!(resp.followers[0].user_id, "u1");
    }

    #[tokio::test]
    async fn test_get_user_profile() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/getprofile"))
            .and(query_param("access_token", "test-token"))
            .and(query_param("user_id", "u1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "u1",
                "display_name": "Alice",
                "avatar": "https://example.com/avatar.jpg"
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client.get_user_profile("u1").await.unwrap();
        assert_eq!(resp["display_name"], "Alice");
    }

    #[tokio::test]
    async fn test_broadcast() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/broadcast"))
            .and(query_param("access_token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "broadcast_id": "bc-1",
                "error": 0
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client
            .broadcast(serde_json::json!({ "text": "hello all" }), None)
            .await
            .unwrap();
        assert_eq!(resp["broadcast_id"], "bc-1");
    }

    #[tokio::test]
    async fn test_query_message() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/getmessagestatus"))
            .and(query_param("access_token", "test-token"))
            .and(query_param("msg_id", "msg-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message_id": "msg-123",
                "status": "delivered"
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client.query_message("msg-123").await.unwrap();
        assert_eq!(resp["status"], "delivered");
    }

    #[tokio::test]
    async fn test_tag_user() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/tag/user"))
            .and(query_param("access_token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "error": 0
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client.tag_user("u1", "vip").await.unwrap();
        assert_eq!(resp["error"], 0);
    }

    #[tokio::test]
    async fn test_get_oa_profile() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/getoa"))
            .and(query_param("access_token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "oa_id": "oa-1",
                "name": "Test OA"
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client.get_oa_profile().await.unwrap();
        assert_eq!(resp["name"], "Test OA");
    }

    #[tokio::test]
    async fn test_set_webhook() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/setwebhook"))
            .and(query_param("access_token", "test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "error": 0
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let resp = client.set_webhook("https://example.com/webhook").await.unwrap();
        assert_eq!(resp["error"], 0);
    }
}
