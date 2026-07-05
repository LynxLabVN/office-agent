use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::client::{MetaClient, Platform};

#[derive(Clone)]
pub struct MetaServer {
    pub client: MetaClient,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReelResponse {
    media_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReplyResponse {
    reply_id: String,
}

fn parse_platform(s: &str) -> Result<Platform, String> {
    match s.to_lowercase().as_str() {
        "instagram" => Ok(Platform::Instagram),
        "facebook" => Ok(Platform::Facebook),
        _ => Err(format!("unsupported platform: {}", s)),
    }
}

#[tool(tool_box)]
impl MetaServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-social-meta".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "post_reel", description = "Publish a reel to Instagram or Facebook")]
    pub async fn post_reel(
        &self,
        #[tool(param)] video_path: String,
        #[tool(param)] caption: String,
        #[tool(param)] platform: String,
        #[tool(param)] thumb: Option<String>,
    ) -> Result<String, String> {
        let platform = parse_platform(&platform)?;
        let resp = self
            .client
            .post_reel(&video_path, &caption, platform, thumb.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&ReelResponse {
            media_id: resp.media_id,
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "list_comments", description = "List comments on an Instagram media or Facebook post")]
    pub async fn list_comments(
        &self,
        #[tool(param)] media_id: String,
        #[tool(param)] platform: String,
    ) -> Result<String, String> {
        let platform = parse_platform(&platform)?;
        let comments = self
            .client
            .list_comments(&media_id, platform)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&comments).map_err(|e| e.to_string())
    }

    #[tool(name = "reply", description = "Reply to a comment on Instagram or Facebook")]
    pub async fn reply(
        &self,
        #[tool(param)] comment_id: String,
        #[tool(param)] platform: String,
        #[tool(param)] text: String,
    ) -> Result<String, String> {
        let platform = parse_platform(&platform)?;
        let resp = self
            .client
            .reply(&comment_id, platform, &text)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&ReplyResponse {
            reply_id: resp.reply_id,
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "get_insights", description = "Get Instagram or Facebook media insights")]
    pub async fn get_insights(
        &self,
        #[tool(param)] media_id: String,
        #[tool(param)] platform: String,
        #[tool(param)] metrics: Vec<String>,
    ) -> Result<String, String> {
        let platform = parse_platform(&platform)?;
        let insights = self
            .client
            .get_insights(&media_id, platform, &metrics)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&insights).map_err(|e| e.to_string())
    }
}

impl ServerHandler for MetaServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-social-meta".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Meta (Instagram/Facebook) MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::MetaClient;
    use crate::oauth::MetaCredentials;

    fn test_server() -> MetaServer {
        MetaServer {
            client: MetaClient::with_base(
                MetaCredentials {
                    app_id: "app-id".to_string(),
                    app_secret: "app-secret".to_string(),
                    page_access_token: "test-token".to_string(),
                    ig_user_id: "ig-user-123".to_string(),
                    page_id: "page-123".to_string(),
                },
                "http://localhost".to_string(),
            ),
        }
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-social-meta"));
    }
}
