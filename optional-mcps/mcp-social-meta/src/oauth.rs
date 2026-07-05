use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct MetaCredentials {
    #[allow(dead_code)]
    pub app_id: String,
    #[allow(dead_code)]
    pub app_secret: String,
    pub page_access_token: String,
    pub ig_user_id: String,
    pub page_id: String,
}

pub fn load_credentials() -> Result<MetaCredentials> {
    Ok(MetaCredentials {
        app_id: std::env::var("META_APP_ID").context("META_APP_ID not set")?,
        app_secret: std::env::var("META_APP_SECRET").context("META_APP_SECRET not set")?,
        page_access_token: std::env::var("META_PAGE_ACCESS_TOKEN")
            .context("META_PAGE_ACCESS_TOKEN not set")?,
        ig_user_id: std::env::var("META_IG_USER_ID").context("META_IG_USER_ID not set")?,
        page_id: std::env::var("META_PAGE_ID").unwrap_or_else(|_| "me".to_string()),
    })
}
