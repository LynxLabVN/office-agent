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

pub fn log(action: &str, target: &str, before: Option<serde_json::Value>, after: Option<serde_json::Value>) {
    let entry = AuditEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        actor: "agent",
        action,
        target,
        before,
        after,
        meta: Some(serde_json::json!({"server": "mcp-hr-data"})),
    };
    if let Ok(line) = serde_json::to_string(&entry) {
        let path = audit_log_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{}", line);
        }
    }
}
