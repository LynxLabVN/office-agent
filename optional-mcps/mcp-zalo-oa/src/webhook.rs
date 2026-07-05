use anyhow::Result;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, warn};

const DEFAULT_WEBHOOK_PORT: u16 = 8080;
const COMMS_LOG_PATH: &str = ".hermes/data/comms_log.json";

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct WebhookState {
    pub secret: String,
}

pub async fn run_webhook_server(port: u16, secret: String) -> anyhow::Result<()> {
    let state = Arc::new(WebhookState { secret });
    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    debug!("Zalo OA webhook server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn webhook_port() -> u16 {
    std::env::var("ZALO_OA_WEBHOOK_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_WEBHOOK_PORT)
}

pub fn webhook_secret() -> Option<String> {
    std::env::var("ZALO_OA_WEBHOOK_SECRET").ok()
}

async fn webhook_handler(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let signature = headers
        .get("X-Zalo-Signature")
        .or_else(|| headers.get("x-zalo-signature"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if !verify_signature(&state.secret, &body, &signature) {
        warn!("Zalo webhook signature verification failed");
        return (StatusCode::FORBIDDEN, json!({ "error": "invalid signature" }).to_string());
    }

    let event: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                json!({ "error": format!("invalid json: {}", e) }).to_string(),
            );
        }
    };

    match process_inbound(event).await {
        Ok(result) => (StatusCode::OK, result.to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": e.to_string() }).to_string(),
        ),
    }
}

pub fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    let Ok(sig_bytes) = hex::decode(signature) else {
        return false;
    };
    expected.as_slice() == sig_bytes.as_slice()
}

pub async fn process_inbound(event: serde_json::Value) -> Result<serde_json::Value> {
    let event_name = event.get("event").and_then(|v| v.as_str()).unwrap_or("unknown");
    let user_id = event
        .get("sender")
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .or_else(|| event.get("user_id").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    let mode = std::env::var("ZALO_OA_REPLY_MODE").unwrap_or_else(|_| "suggest".to_string());
    let can_reply = if user_id.is_empty() {
        false
    } else {
        within_24h(&user_id)
    };

    debug!(
        "process_inbound event={} user_id={} mode={} can_reply={}",
        event_name, user_id, mode, can_reply
    );

    let action = match mode.as_str() {
        "auto" if can_reply => "reply",
        "auto" => "queue",
        "suggest" => "suggest",
        _ => "ignore",
    };

    Ok(json!({
        "event": event_name,
        "user_id": user_id,
        "mode": mode,
        "within_24h": can_reply,
        "action": action
    }))
}

pub fn within_24h(user_id: &str) -> bool {
    let path = comms_log_path();
    if !path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("failed to read comms log: {}", e);
            return false;
        }
    };

    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            warn!("failed to parse comms log: {}", e);
            return false;
        }
    };

    let timestamp_str = if let Some(obj) = value.as_object() {
        obj.get(user_id)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else if let Some(arr) = value.as_array() {
        arr.iter()
            .filter_map(|v| v.as_object())
            .rfind(|o| o.get("user_id").and_then(|u| u.as_str()) == Some(user_id))
            .and_then(|o| o.get("timestamp").and_then(|t| t.as_str()))
            .map(|s| s.to_string())
    } else {
        None
    };

    let Some(ts_str) = timestamp_str else {
        return false;
    };

    let timestamp = match DateTime::parse_from_rfc3339(&ts_str) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => match chrono::NaiveDateTime::parse_from_str(&ts_str, "%Y-%m-%d %H:%M:%S") {
            Ok(ndt) => ndt.and_utc(),
            Err(e) => {
                warn!("failed to parse timestamp '{}': {}", ts_str, e);
                return false;
            }
        }
    };

    Utc::now().signed_duration_since(timestamp).num_seconds() < 24 * 60 * 60
}

fn comms_log_path() -> PathBuf {
    if let Ok(path) = std::env::var("ZALO_OA_COMMS_LOG") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(COMMS_LOG_PATH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature_valid() {
        let secret = "my-secret";
        let body = b"{\"event\":\"message\"}";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(verify_signature(secret, body, &signature));
    }

    #[test]
    fn test_verify_signature_invalid() {
        let secret = "my-secret";
        let body = b"{\"event\":\"message\"}";
        assert!(!verify_signature(secret, body, "deadbeef"));
        assert!(!verify_signature("wrong-secret", body, "deadbeef"));
    }

    #[tokio::test]
    async fn test_process_inbound_suggest_mode() {
        std::env::remove_var("ZALO_OA_REPLY_MODE");
        let event = json!({
            "event": "message",
            "sender": { "id": "user-1" }
        });
        let result = process_inbound(event).await.unwrap();
        assert_eq!(result["event"], "message");
        assert_eq!(result["user_id"], "user-1");
        assert_eq!(result["mode"], "suggest");
        assert_eq!(result["action"], "suggest");
    }

    #[tokio::test]
    async fn test_process_inbound_auto_mode_no_log() {
        std::env::remove_var("ZALO_OA_COMMS_LOG");
        std::env::set_var("ZALO_OA_REPLY_MODE", "auto");
        let event = json!({
            "event": "message",
            "user_id": "user-2"
        });
        let result = process_inbound(event).await.unwrap();
        assert_eq!(result["mode"], "auto");
        assert_eq!(result["within_24h"], false);
        assert_eq!(result["action"], "queue");
    }

    #[test]
    fn test_within_24h_object_format() {
        std::env::remove_var("ZALO_OA_COMMS_LOG");
        let tmp = temp_log_path("object");
        let now = Utc::now().to_rfc3339();
        std::fs::write(&tmp, json!({ "user-3": now }).to_string()).unwrap();
        std::env::set_var("ZALO_OA_COMMS_LOG", tmp.to_str().unwrap());
        assert!(within_24h("user-3"));
        assert!(!within_24h("user-4"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_within_24h_array_format() {
        std::env::remove_var("ZALO_OA_COMMS_LOG");
        let tmp = temp_log_path("array");
        let now = Utc::now().to_rfc3339();
        std::fs::write(
            &tmp,
            json!([
                { "user_id": "user-5", "timestamp": now }
            ])
            .to_string(),
        )
        .unwrap();
        std::env::set_var("ZALO_OA_COMMS_LOG", tmp.to_str().unwrap());
        assert!(within_24h("user-5"));
        assert!(!within_24h("user-6"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_within_24h_old() {
        std::env::remove_var("ZALO_OA_COMMS_LOG");
        let tmp = temp_log_path("old");
        let old = Utc::now() - chrono::Duration::hours(25);
        std::fs::write(&tmp, json!({ "user-7": old.to_rfc3339() }).to_string()).unwrap();
        std::env::set_var("ZALO_OA_COMMS_LOG", tmp.to_str().unwrap());
        assert!(!within_24h("user-7"));
        let _ = std::fs::remove_file(&tmp);
    }

    fn temp_log_path(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("comms-log-{}-{}.json", suffix, std::process::id()))
    }
}
