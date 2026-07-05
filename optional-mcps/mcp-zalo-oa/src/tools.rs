use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::client::ZaloOaClient;

#[derive(Clone)]
pub struct ZaloOaServer {
    pub client: ZaloOaClient,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MessageIdResponse {
    message_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct BroadcastIdResponse {
    broadcast_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OkResponse {
    ok: bool,
}

fn ok_json() -> Result<String, String> {
    serde_json::to_string(&OkResponse { ok: true }).map_err(|e| e.to_string())
}

#[tool(tool_box)]
impl ZaloOaServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-zalo-oa".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "send_oa_message", description = "Send a text message to a Zalo OA user")]
    pub async fn send_oa_message(
        &self,
        #[tool(param)] user_id: String,
        #[tool(param)] text: String,
    ) -> Result<String, String> {
        let resp = self
            .client
            .send_oa_message(&user_id, &text)
            .await
            .map_err(|e| e.to_string())?;
        let message_id = resp
            .get("message_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        serde_json::to_string(&MessageIdResponse { message_id }).map_err(|e| e.to_string())
    }

    #[tool(
        name = "send_oa_message_template",
        description = "Send a template message to a Zalo OA user"
    )]
    pub async fn send_oa_message_template(
        &self,
        #[tool(param)] user_id: String,
        #[tool(param)] template_id: String,
        #[tool(param)] template_data: String,
    ) -> Result<String, String> {
        let data: serde_json::Value =
            serde_json::from_str(&template_data).map_err(|e| e.to_string())?;
        let resp = self
            .client
            .send_oa_message_template(&user_id, &template_id, data)
            .await
            .map_err(|e| e.to_string())?;
        let message_id = resp
            .get("message_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        serde_json::to_string(&MessageIdResponse { message_id }).map_err(|e| e.to_string())
    }

    #[tool(
        name = "send_oa_attachment",
        description = "Send an attachment (image/file) to a Zalo OA user"
    )]
    pub async fn send_oa_attachment(
        &self,
        #[tool(param)] user_id: String,
        #[tool(param)] attachment_type: String,
        #[tool(param)] url: String,
        #[tool(param)] caption: Option<String>,
    ) -> Result<String, String> {
        let resp = self
            .client
            .send_oa_attachment(&user_id, &attachment_type, &url, caption.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        let message_id = resp
            .get("message_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        serde_json::to_string(&MessageIdResponse { message_id }).map_err(|e| e.to_string())
    }

    #[tool(name = "list_followers", description = "List Zalo OA followers")]
    pub async fn list_followers(
        &self,
        #[tool(param)] offset: u32,
        #[tool(param)] count: u32,
    ) -> Result<String, String> {
        let resp = self
            .client
            .list_followers(offset, count)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&resp).map_err(|e| e.to_string())
    }

    #[tool(name = "get_user_profile", description = "Get a Zalo OA user's profile")]
    pub async fn get_user_profile(
        &self,
        #[tool(param)] user_id: String,
    ) -> Result<String, String> {
        let resp = self
            .client
            .get_user_profile(&user_id)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&resp).map_err(|e| e.to_string())
    }

    #[tool(name = "broadcast", description = "Broadcast a message to Zalo OA followers")]
    pub async fn broadcast(
        &self,
        #[tool(param)] message: String,
        #[tool(param)] segment: Option<String>,
    ) -> Result<String, String> {
        let message_value: serde_json::Value =
            serde_json::from_str(&message).map_err(|e| e.to_string())?;
        let segment_value = segment
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| e.to_string())?;
        let resp = self
            .client
            .broadcast(message_value, segment_value)
            .await
            .map_err(|e| e.to_string())?;
        let broadcast_id = resp
            .get("broadcast_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        serde_json::to_string(&BroadcastIdResponse { broadcast_id }).map_err(|e| e.to_string())
    }

    #[tool(name = "query_message", description = "Query status of a Zalo OA message")]
    pub async fn query_message(
        &self,
        #[tool(param)] message_id: String,
    ) -> Result<String, String> {
        let resp = self
            .client
            .query_message(&message_id)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&resp).map_err(|e| e.to_string())
    }

    #[tool(name = "tag_user", description = "Tag a Zalo OA user")]
    pub async fn tag_user(
        &self,
        #[tool(param)] user_id: String,
        #[tool(param)] tag: String,
    ) -> Result<String, String> {
        self.client
            .tag_user(&user_id, &tag)
            .await
            .map_err(|e| e.to_string())?;
        ok_json()
    }

    #[tool(name = "get_oa_profile", description = "Get the Zalo Official Account profile")]
    pub async fn get_oa_profile(&self) -> Result<String, String> {
        let resp = self
            .client
            .get_oa_profile()
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&resp).map_err(|e| e.to_string())
    }

    #[tool(name = "set_webhook", description = "Configure the Zalo OA webhook URL")]
    pub async fn set_webhook(&self, #[tool(param)] url: String) -> Result<String, String> {
        self.client
            .set_webhook(&url)
            .await
            .map_err(|e| e.to_string())?;
        ok_json()
    }
}

impl ServerHandler for ZaloOaServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-zalo-oa".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Zalo Official Account MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ZaloOaClient;

    fn test_server() -> ZaloOaServer {
        ZaloOaServer {
            client: ZaloOaClient::with_base("test-token".to_string(), "http://localhost".to_string()),
        }
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-zalo-oa"));
    }
}
