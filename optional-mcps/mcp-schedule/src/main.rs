use rmcp::serve_server;

mod client;
mod tools;

use client::CalComClient;
use tools::ScheduleServer;

fn api_key() -> anyhow::Result<String> {
    std::env::var("CALCOM_API_KEY").map_err(|_| anyhow::anyhow!("CALCOM_API_KEY not set"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let api_key = api_key()?;
    let base_url = std::env::var("CALCOM_API_URL").unwrap_or_else(|_| "https://api.cal.com/v1".to_string());
    let client = CalComClient::with_base(api_key, base_url);
    let server = ScheduleServer { client };

    let service = serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
