use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::env;

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Clone)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
}

pub fn load_credentials() -> Result<Credentials> {
    Ok(Credentials {
        client_id: env::var("YOUTUBE_CLIENT_ID").map_err(|_| anyhow::anyhow!("YOUTUBE_CLIENT_ID not set"))?,
        client_secret: env::var("YOUTUBE_CLIENT_SECRET").map_err(|_| anyhow::anyhow!("YOUTUBE_CLIENT_SECRET not set"))?,
        refresh_token: env::var("YOUTUBE_REFRESH_TOKEN").map_err(|_| anyhow::anyhow!("YOUTUBE_REFRESH_TOKEN not set"))?,
    })
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub async fn refresh_access_token(credentials: &Credentials) -> Result<String> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", credentials.client_id.as_str()),
        ("client_secret", credentials.client_secret.as_str()),
        ("refresh_token", credentials.refresh_token.as_str()),
        ("grant_type", "refresh_token"),
    ];

    let resp = client
        .post(GOOGLE_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .context("token refresh request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("token refresh failed {}: {}", status, text);
    }

    let token: TokenResponse = resp
        .json()
        .await
        .context("failed to decode token response")?;
    Ok(token.access_token)
}
