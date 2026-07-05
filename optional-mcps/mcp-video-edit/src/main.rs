use rmcp::serve_server;

mod ffmpeg;
mod tools;

use tools::VideoEditServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    ffmpeg::ensure_workspace()?;

    let service = serve_server(VideoEditServer, (tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
