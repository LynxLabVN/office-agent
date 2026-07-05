use rmcp::serve_server;
use std::path::PathBuf;

mod audit;
mod db;
mod tools;

use tools::HrDataServer;

fn default_db_path() -> PathBuf {
    if let Ok(path) = std::env::var("HR_DB") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hermes/data/hr.db")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let db_path = default_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = db::open_db(&db_path)?;
    let server = HrDataServer {
        db: std::sync::Arc::new(std::sync::Mutex::new(conn)),
    };

    let service = serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
