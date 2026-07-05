use rmcp::serve_server;

mod client;
mod tools;

use client::TrendClient;
use tools::TrendResearchServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let client = TrendClient::new()?;
    let server = TrendResearchServer { client };

    let service = serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
