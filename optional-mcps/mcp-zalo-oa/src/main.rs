use rmcp::serve_server;

mod audit;
mod client;
mod tools;
mod webhook;

use client::ZaloOaClient;
use tools::ZaloOaServer;

fn access_token() -> anyhow::Result<String> {
    std::env::var("ZALO_OA_ACCESS_TOKEN")
        .map_err(|_| anyhow::anyhow!("ZALO_OA_ACCESS_TOKEN not set"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let token = access_token()?;
    let base_url = std::env::var("ZALO_OA_API_URL")
        .unwrap_or_else(|_| "https://openapi.zalo.me/v2.0/officialaccount".to_string());
    let client = ZaloOaClient::with_base(token, base_url);
    let server = ZaloOaServer { client };

    if std::env::var("ZALO_OA_WEBHOOK_PORT").is_ok() {
        let port = webhook::webhook_port();
        if let Some(secret) = webhook::webhook_secret() {
            tokio::spawn(async move {
                if let Err(e) = webhook::run_webhook_server(port, secret).await {
                    tracing::error!("webhook server error: {}", e);
                }
            });
        } else {
            tracing::warn!(
                "ZALO_OA_WEBHOOK_PORT is set but ZALO_OA_WEBHOOK_SECRET is missing; webhook server not started"
            );
        }
    }

    let service = serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
