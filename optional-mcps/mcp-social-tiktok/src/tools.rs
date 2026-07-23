use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::client::TikTokClient;

#[derive(Clone)]
pub struct TikTokServer {
    pub client: TikTokClient,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReplyResponse {
    comment_id: String,
}

#[tool(tool_box)]
impl TikTokServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-social-tiktok".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(
        name = "post_video",
        description = "Upload and publish a video to TikTok"
    )]
    pub async fn post_video(
        &self,
        #[tool(param)] video_path: String,
        #[tool(param)] title: String,
        #[tool(param)] privacy_level: String,
        #[tool(param)] tags: Option<Vec<String>>,
    ) -> Result<String, String> {
        let tags = tags.unwrap_or_default();
        let posted = self
            .client
            .post_video(Path::new(&video_path), &title, &privacy_level, &tags)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&posted).map_err(|e| e.to_string())
    }

    #[tool(name = "get_metrics", description = "Get TikTok video metrics")]
    pub async fn get_metrics(&self, #[tool(param)] video_id: String) -> Result<String, String> {
        let metrics = self
            .client
            .get_metrics(&video_id)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&metrics).map_err(|e| e.to_string())
    }

    #[tool(name = "list_comments", description = "List comments on a TikTok video")]
    pub async fn list_comments(
        &self,
        #[tool(param)] video_id: String,
        #[tool(param)] max_results: Option<u32>,
    ) -> Result<String, String> {
        let comments = self
            .client
            .list_comments(&video_id, max_results)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&comments).map_err(|e| e.to_string())
    }

    #[tool(name = "reply_comment", description = "Reply to a TikTok comment")]
    pub async fn reply_comment(
        &self,
        #[tool(param)] comment_id: String,
        #[tool(param)] text: String,
    ) -> Result<String, String> {
        let id = self
            .client
            .reply_comment(&comment_id, &text)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&ReplyResponse {
            comment_id: id,
        })
        .map_err(|e| e.to_string())
    }
}

impl ServerHandler for TikTokServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-social-tiktok".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("TikTok social MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::TikTokClient;
    use crate::oauth::TikTokOAuth;

    fn test_server() -> TikTokServer {
        TikTokServer {
            client: TikTokClient::with_base(
                TikTokOAuth {
                    client_key: "test-key".to_string(),
                    client_secret: "test-secret".to_string(),
                    access_token: "test-token".to_string(),
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
        assert!(out.contains("mcp-social-tiktok"));
    }
}
