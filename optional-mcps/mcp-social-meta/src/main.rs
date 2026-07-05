use rmcp::serve_server;

mod client;
mod oauth;
mod tools;
mod upload_state;

use client::MetaClient;
use tools::MetaServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let creds = oauth::load_credentials()?;
    let base_url = std::env::var("META_BASE_URL")
        .unwrap_or_else(|_| "https://graph.facebook.com/v22.0".to_string());
    let client = MetaClient::with_base(creds, base_url);
    let server = MetaServer { client };

    let service = serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
