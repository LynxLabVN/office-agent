use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::client::CalComClient;

#[derive(Clone)]
pub struct ScheduleServer {
    pub client: CalComClient,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EventTypeResponse {
    event_type_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BookingResponse {
    booking_id: u64,
    uid: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OkResponse {
    ok: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct BookingRow {
    id: u64,
    title: String,
    start: String,
    end: String,
    attendee_name: String,
    attendee_email: String,
}

#[tool(tool_box)]
impl ScheduleServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-schedule".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "create_event_type", description = "Create a Cal.com event type")]
    pub async fn create_event_type(
        &self,
        #[tool(param)] title: String,
        #[tool(param)] length_min: u32,
        #[tool(param)] slug: String,
    ) -> Result<String, String> {
        let et = self
            .client
            .create_event_type(&title, length_min, &slug)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&EventTypeResponse { event_type_id: et.id }).map_err(|e| e.to_string())
    }

    #[tool(name = "list_slots", description = "List available Cal.com slots for an event type")]
    pub async fn list_slots(
        &self,
        #[tool(param)] event_type_id: String,
        #[tool(param)] date_from: String,
        #[tool(param)] date_to: String,
    ) -> Result<String, String> {
        let resp = self
            .client
            .list_slots(&event_type_id, &date_from, &date_to)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&resp.slots).map_err(|e| e.to_string())
    }

    #[tool(name = "book_slot", description = "Book a Cal.com slot")]
    pub async fn book_slot(
        &self,
        #[tool(param)] event_type_id: u64,
        #[tool(param)] start: String,
        #[tool(param)] name: String,
        #[tool(param)] email: String,
        #[tool(param)] notes: Option<String>,
    ) -> Result<String, String> {
        let b = self
            .client
            .book_slot(event_type_id, &start, &name, &email, notes.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&BookingResponse {
            booking_id: b.id,
            uid: b.uid,
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "list_bookings", description = "List Cal.com bookings")]
    pub async fn list_bookings(
        &self,
        #[tool(param)] status: Option<String>,
    ) -> Result<String, String> {
        let bookings = self
            .client
            .list_bookings(status.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        let rows: Vec<BookingRow> = bookings
            .into_iter()
            .map(|b| {
                let attendee = b.attendees.into_iter().next();
                BookingRow {
                    id: b.id,
                    title: b.title,
                    start: b.start_time,
                    end: b.end_time,
                    attendee_name: attendee.as_ref().map(|a| a.name.clone()).unwrap_or_default(),
                    attendee_email: attendee.map(|a| a.email).unwrap_or_default(),
                }
            })
            .collect();
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    }

    #[tool(name = "cancel_booking", description = "Cancel a Cal.com booking")]
    pub async fn cancel_booking(
        &self,
        #[tool(param)] booking_id: String,
        #[tool(param)] reason: Option<String>,
    ) -> Result<String, String> {
        self.client
            .cancel_booking(&booking_id, reason.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&OkResponse { ok: true }).map_err(|e| e.to_string())
    }

    #[tool(name = "send_invite", description = "Send a booking invite via email or zalo")]
    pub async fn send_invite(
        &self,
        #[tool(param)] booking_id: String,
        #[tool(param)] channel: String,
    ) -> Result<String, String> {
        self.client
            .send_invite(&booking_id, &channel)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&OkResponse { ok: true }).map_err(|e| e.to_string())
    }
}

impl ServerHandler for ScheduleServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-schedule".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Cal.com scheduling MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::CalComClient;

    fn test_server() -> ScheduleServer {
        ScheduleServer {
            client: CalComClient::with_base("test-key".to_string(), "http://localhost".to_string()),
        }
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-schedule"));
    }
}
