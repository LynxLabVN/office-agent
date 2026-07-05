use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::client::YouTubeClient;

#[derive(Clone)]
pub struct YouTubeServer {
    pub client: YouTubeClient,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UploadResponse {
    video_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReplyResponse {
    comment_id: String,
}

#[tool(tool_box)]
impl YouTubeServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-social-youtube".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "upload", description = "Upload a video to YouTube")]
    pub async fn upload(
        &self,
        #[tool(param)] video_path: String,
        #[tool(param)] title: String,
        #[tool(param)] description: String,
        #[tool(param)] tags: Option<Vec<String>>,
        #[tool(param)] category_id: Option<u32>,
        #[tool(param)] privacy: Option<String>,
    ) -> Result<String, String> {
        let tags = tags.unwrap_or_default();
        let category_id = category_id.unwrap_or(22);
        let privacy = privacy.unwrap_or_else(|| "private".to_string());
        let id = self
            .client
            .upload(&video_path, &title, &description, tags, category_id, &privacy)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&UploadResponse { video_id: id }).map_err(|e| e.to_string())
    }

    #[tool(name = "resume_upload", description = "Resume an interrupted YouTube upload from saved state")]
    pub async fn resume_upload(
        &self,
        #[tool(param)] video_path: String,
    ) -> Result<String, String> {
        let id = self
            .client
            .resume_upload(&video_path)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&UploadResponse { video_id: id }).map_err(|e| e.to_string())
    }

    #[tool(name = "list_comments", description = "List top-level comments for a YouTube video")]
    pub async fn list_comments(
        &self,
        #[tool(param)] video_id: String,
        #[tool(param)] max_results: Option<u32>,
    ) -> Result<String, String> {
        let comments = self
            .client
            .list_comments(&video_id, max_results.unwrap_or(20))
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&comments).map_err(|e| e.to_string())
    }

    #[tool(name = "reply_comment", description = "Reply to a YouTube comment")]
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
        serde_json::to_string(&ReplyResponse { comment_id: id }).map_err(|e| e.to_string())
    }

    #[tool(name = "get_stats", description = "Get public statistics for a YouTube video")]
    pub async fn get_stats(
        &self,
        #[tool(param)] video_id: String,
    ) -> Result<String, String> {
        let stats = self
            .client
            .get_stats(&video_id)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&stats).map_err(|e| e.to_string())
    }

    #[tool(name = "get_video_analytics", description = "Get YouTube Analytics for a video")]
    pub async fn get_video_analytics(
        &self,
        #[tool(param)] video_id: String,
        #[tool(param)] start_date: String,
        #[tool(param)] end_date: String,
    ) -> Result<String, String> {
        let data = self
            .client
            .get_video_analytics(&video_id, &start_date, &end_date)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&data).map_err(|e| e.to_string())
    }
}

impl ServerHandler for YouTubeServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-social-youtube".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("YouTube social MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::YouTubeClient;
    use crate::oauth::Credentials;
    use std::path::PathBuf;

    fn test_server() -> YouTubeServer {
        let client = YouTubeClient::with_base_and_quota(
            Credentials {
                client_id: "id".to_string(),
                client_secret: "secret".to_string(),
                refresh_token: "rt".to_string(),
            },
            "http://localhost".to_string(),
            PathBuf::from("/tmp/yt_quota_tools_test.json"),
        );
        YouTubeServer { client }
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-social-youtube"));
    }
}
