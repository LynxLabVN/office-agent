use anyhow::{Context, Result};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

#[allow(dead_code)]
const DEFAULT_BASE_URL: &str = "https://api.cal.com/v1";
const RATE_LIMIT_PER_MIN: u32 = 120;

#[derive(Clone)]
pub struct CalComClient {
    inner: Client,
    api_key: String,
    base_url: String,
    bucket: Arc<Mutex<TokenBucket>>,
}

struct TokenBucket {
    tokens: f64,
    last: Instant,
    rate_per_second: f64,
    capacity: f64,
}

impl TokenBucket {
    fn new(capacity: u32) -> Self {
        Self {
            tokens: capacity as f64,
            last: Instant::now(),
            rate_per_second: capacity as f64 / 60.0,
            capacity: capacity as f64,
        }
    }

    async fn acquire(&mut self) -> Duration {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate_per_second).min(self.capacity);
        self.last = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Duration::ZERO
        } else {
            let needed = 1.0 - self.tokens;
            let wait = Duration::from_secs_f64(needed / self.rate_per_second);
            self.tokens = 0.0;
            wait
        }
    }
}

impl CalComClient {
    #[allow(dead_code)]
    pub fn new(api_key: String) -> Self {
        Self::with_base(api_key, DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base(api_key: String, base_url: String) -> Self {
        Self {
            inner: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("build reqwest client"),
            api_key,
            base_url,
            bucket: Arc::new(Mutex::new(TokenBucket::new(RATE_LIMIT_PER_MIN))),
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
            .query(&[("apiKey", self.api_key.as_str())])
    }

    async fn send(&self, req: RequestBuilder) -> Result<Response> {
        let wait = self.bucket.lock().await.acquire().await;
        if !wait.is_zero() {
            debug!("rate limiter sleeping {:?}", wait);
            tokio::time::sleep(wait).await;
        }
        let resp = req.send().await.context("HTTP request failed")?;
        Ok(resp)
    }

    pub async fn create_event_type(
        &self,
        title: &str,
        length_min: u32,
        slug: &str,
    ) -> Result<EventType> {
        let body = serde_json::json!({
            "title": title,
            "length": length_min,
            "slug": slug,
        });
        let resp = self
            .send(self.request(Method::POST, "/event-types").json(&body))
            .await?;
        check_status(resp).await
    }

    pub async fn list_slots(
        &self,
        event_type_id: &str,
        date_from: &str,
        date_to: &str,
    ) -> Result<SlotsResponse> {
        let resp = self
            .send(
                self.request(Method::GET, "/slots").query(&[
                    ("eventTypeId", event_type_id),
                    ("startTime", date_from),
                    ("endTime", date_to),
                ]),
            )
            .await?;
        check_status(resp).await
    }

    pub async fn book_slot(
        &self,
        event_type_id: u64,
        start: &str,
        name: &str,
        email: &str,
        notes: Option<&str>,
    ) -> Result<Booking> {
        let mut body = serde_json::json!({
            "eventTypeId": event_type_id,
            "start": start,
            "responses": {
                "name": name,
                "email": email,
            },
        });
        if let Some(n) = notes {
            body["responses"]["notes"] = serde_json::Value::String(n.to_string());
        }
        let resp = self
            .send(self.request(Method::POST, "/bookings").json(&body))
            .await?;
        check_status(resp).await
    }

    pub async fn list_bookings(&self, status: Option<&str>) -> Result<Vec<Booking>> {
        let mut req = self.request(Method::GET, "/bookings");
        if let Some(s) = status {
            req = req.query(&[("status", s)]);
        }
        let resp = self.send(req).await?;
        #[derive(Deserialize)]
        struct BookingsResponse {
            bookings: Vec<Booking>,
        }
        let parsed: BookingsResponse = check_status(resp).await?;
        Ok(parsed.bookings)
    }

    pub async fn cancel_booking(&self, booking_id: &str, reason: Option<&str>) -> Result<()> {
        let mut body = serde_json::json!({});
        if let Some(r) = reason {
            body["reason"] = serde_json::Value::String(r.to_string());
        }
        let resp = self
            .send(
                self.request(Method::DELETE, &format!("/bookings/{}", booking_id))
                    .json(&body),
            )
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Cal.com cancel failed {}: {}", status, text))
        }
    }

    pub async fn send_invite(&self, booking_id: &str, channel: &str) -> Result<()> {
        // Placeholder: Hermes gateway integration would go here.
        // We verify inputs and return OK for mocked/unit-test path.
        if !matches!(channel, "email" | "zalo") {
            anyhow::bail!("unsupported invite channel: {}", channel);
        }
        debug!("send_invite booking={} channel={}", booking_id, channel);
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
            warn!("Cal.com rate limited");
        }
        Err(anyhow::anyhow!("Cal.com API error {}: {}", status, text))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventType {
    pub id: u64,
    pub title: String,
    pub slug: String,
    pub length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub start: String,
    pub end: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotsResponse {
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Booking {
    pub id: u64,
    pub uid: String,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    #[serde(default)]
    pub attendees: Vec<Attendee>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    pub name: String,
    pub email: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_create_event_type() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/event-types"))
            .and(query_param("apiKey", "test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42,
                "title": "Technical Interview",
                "slug": "tech-interview",
                "length": 60,
            })))
            .mount(&server)
            .await;

        let client = CalComClient::with_base("test-key".to_string(), server.uri());
        let et = client.create_event_type("Technical Interview", 60, "tech-interview").await.unwrap();
        assert_eq!(et.id, 42);
        assert_eq!(et.slug, "tech-interview");
    }

    #[tokio::test]
    async fn test_list_slots() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/slots"))
            .and(query_param("apiKey", "test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "slots": [{"start":"2026-07-10T10:00:00Z","end":"2026-07-10T11:00:00Z","available":true}]
            })))
            .mount(&server)
            .await;

        let client = CalComClient::with_base("test-key".to_string(), server.uri());
        let resp = client.list_slots("7", "2026-07-10", "2026-07-11").await.unwrap();
        assert_eq!(resp.slots.len(), 1);
        assert!(resp.slots[0].available);
    }

    #[tokio::test]
    async fn test_book_slot() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/bookings"))
            .and(query_param("apiKey", "test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 99,
                "uid": "abc-123",
                "title": "Technical Interview",
                "start_time": "2026-07-10T10:00:00Z",
                "end_time": "2026-07-10T11:00:00Z",
                "attendees": [{"name":"Nguyen Van A","email":"a@b.com"}],
            })))
            .mount(&server)
            .await;

        let client = CalComClient::with_base("test-key".to_string(), server.uri());
        let b = client.book_slot(7, "2026-07-10T10:00:00Z", "Nguyen Van A", "a@b.com", None).await.unwrap();
        assert_eq!(b.id, 99);
        assert_eq!(b.uid, "abc-123");
    }
}
