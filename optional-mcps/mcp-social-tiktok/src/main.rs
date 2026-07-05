use rmcp::serve_server;

mod client;
mod oauth;
mod tools;

use client::{TikTokClient, DEFAULT_BASE_URL};
use oauth::TikTokOAuth;
use tools::TikTokServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let auth = TikTokOAuth::from_env()?;
    let base_url = std::env::var("TIKTOK_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let client = TikTokClient::with_base(auth, base_url);
    let server = TikTokServer { client };

    let service = serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
