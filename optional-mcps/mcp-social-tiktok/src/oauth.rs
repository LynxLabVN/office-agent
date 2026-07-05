use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct TikTokOAuth {
    #[allow(dead_code)]
    pub client_key: String,
    #[allow(dead_code)]
    pub client_secret: String,
    pub access_token: String,
}

impl TikTokOAuth {
    pub fn from_env() -> Result<Self> {
        let client_key = std::env::var("TIKTOK_CLIENT_KEY")
            .context("TIKTOK_CLIENT_KEY environment variable not set")?;
        let client_secret = std::env::var("TIKTOK_CLIENT_SECRET")
            .context("TIKTOK_CLIENT_SECRET environment variable not set")?;
        let access_token = std::env::var("TIKTOK_ACCESS_TOKEN")
            .context("TIKTOK_ACCESS_TOKEN environment variable not set")?;
        Ok(Self {
            client_key,
            client_secret,
            access_token,
        })
    }

    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}
