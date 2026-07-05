use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct AuditEntry<'a> {
    ts: String,
    actor: &'a str,
    action: &'a str,
    target: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<serde_json::Value>,
}

fn audit_log_path() -> PathBuf {
    if let Ok(home) = std::env::var("HERMES_HOME") {
        PathBuf::from(home).join("data").join("audit.log")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".hermes").join("data").join("audit.log")
    } else {
        PathBuf::from("audit.log")
    }
}

pub fn log_pii_access(actor: &str, target: &str, data_type: &str) {
    let entry = AuditEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        actor,
        action: "pii_access",
        target,
        before: None,
        after: Some(serde_json::json!({"data_type": data_type })),
        meta: Some(serde_json::json!({"server": "mcp-zalo-oa", "tool": "get_user_profile" })),
    };
    if let Ok(line) = serde_json::to_string(&entry) {
        let path = audit_log_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        use std::io::Write;
        let mut file = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let _ = writeln!(file, "{}", line);
    }
}
