use rmcp::serve_server;

mod client;
mod oauth;
mod tools;

use client::YouTubeClient;
use tools::YouTubeServer;

fn base_url() -> String {
    std::env::var("YOUTUBE_API_URL")
        .unwrap_or_else(|_| "https://www.googleapis.com".to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let credentials = oauth::load_credentials()?;
    let base_url = base_url();
    let client = YouTubeClient::with_base(credentials, base_url)?;
    let server = YouTubeServer { client };

    let service = serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
