use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::client::TrendClient;

#[derive(Clone)]
pub struct TrendResearchServer {
    pub client: TrendClient,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchHooksResponse {
    hooks: Vec<crate::client::Hook>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TrendingAudioResponse {
    audio: Vec<crate::client::TrendingAudio>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReferenceVideosResponse {
    videos: Vec<crate::client::ReferenceVideo>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorResponse {
    error: String,
}

#[tool(tool_box)]
impl TrendResearchServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-trend-research".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "search_hooks", description = "Search trending hooks for a product category on TikTok + YouTube")]
    pub async fn search_hooks(
        &self,
        #[tool(param)] product_category: String,
        #[tool(param)] region: Option<String>,
    ) -> Result<String, String> {
        match self.client.search_hooks(&product_category, region.as_deref()).await {
            Ok(hooks) => serde_json::to_string(&SearchHooksResponse { hooks })
                .map_err(|e| e.to_string()),
            Err(e) => serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .map_err(|e| e.to_string()),
        }
    }

    #[tool(name = "get_trending_audio", description = "Get trending TikTok audio for a region")]
    pub async fn get_trending_audio(
        &self,
        #[tool(param)] region: Option<String>,
    ) -> Result<String, String> {
        match self.client.get_trending_audio(region.as_deref()).await {
            Ok(audio) => serde_json::to_string(&TrendingAudioResponse { audio })
                .map_err(|e| e.to_string()),
            Err(e) => serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .map_err(|e| e.to_string()),
        }
    }

    #[tool(name = "fetch_reference_videos", description = "Fetch reference videos from YouTube or TikTok")]
    pub async fn fetch_reference_videos(
        &self,
        #[tool(param)] query: String,
        #[tool(param)] platform: String,
        #[tool(param)] limit: Option<u32>,
    ) -> Result<String, String> {
        match self
            .client
            .fetch_reference_videos(&query, &platform, limit)
            .await
        {
            Ok(videos) => serde_json::to_string(&ReferenceVideosResponse { videos })
                .map_err(|e| e.to_string()),
            Err(e) => serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .map_err(|e| e.to_string()),
        }
    }
}

impl ServerHandler for TrendResearchServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-trend-research".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Trend research MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::TrendClient;

    fn test_server() -> TrendResearchServer {
        let client = TrendClient::test_client(
            "http://localhost".to_string(),
            "http://localhost".to_string(),
        );
        TrendResearchServer { client }
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-trend-research"));
    }
}
