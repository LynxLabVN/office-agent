use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct LedgerServer {
    pub db: Arc<Mutex<Connection>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RecordPostResponse {
    post_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PerformanceResponse {
    views: i64,
    likes: i64,
    comments: i64,
    shares: i64,
    watch_time_sec: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct WhatWorkedRow {
    key: String,
    avg_views: f64,
    avg_likes: f64,
    avg_retention: f64,
    count: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct HookLeaderboardRow {
    hook_text: String,
    avg_retention: f64,
    uses: i64,
    last_used_at: Option<String>,
}

#[tool(tool_box)]
impl LedgerServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-ledger".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "record_post", description = "Record a published post and return its ledger id")]
    #[allow(clippy::too_many_arguments)]
    pub fn record_post(
        &self,
        #[tool(param)] piece_id: String,
        #[tool(param)] product_sku: String,
        #[tool(param)] format: String,
        #[tool(param)] platform: String,
        #[tool(param)] platform_post_id: Option<String>,
        #[tool(param)] caption: Option<String>,
        #[tool(param)] hook_text: Option<String>,
    ) -> Result<String, String> {
        let mut conn = self.db.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT INTO posts (piece_id, product_sku, format, platform, platform_post_id, caption, hook_text) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                piece_id,
                product_sku,
                format,
                platform,
                platform_post_id,
                caption,
                hook_text,
            ],
        )
        .map_err(|e| e.to_string())?;

        let post_id = tx.last_insert_rowid();

        // Ensure a metrics row exists for the new post.
        tx.execute(
            "INSERT INTO metrics (post_id) VALUES (?1)",
            params![post_id],
        )
        .map_err(|e| e.to_string())?;

        // Maintain hooks leaderboard.
        if let Some(hook) = &hook_text {
            tx.execute(
                "INSERT INTO hooks (hook_text, product_sku, format, avg_retention, uses, last_used_at) \
                 VALUES (?1, ?2, ?3, 0, 1, datetime('now')) \
                 ON CONFLICT(hook_text) DO UPDATE SET \
                 uses = uses + 1, last_used_at = datetime('now'), product_sku = excluded.product_sku, format = excluded.format",
                params![hook, product_sku, format],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())?;

        crate::audit::log(
            "mcp_call",
            &format!("mcp-ledger.record_post:{}", post_id),
            None,
            Some(serde_json::json!({
                "piece_id": piece_id,
                "product_sku": product_sku,
                "platform": platform,
                "hook_text": hook_text,
            })),
        );

        serde_json::to_string(&RecordPostResponse { post_id }).map_err(|e| e.to_string())
    }

    #[tool(name = "get_performance", description = "Get performance metrics for a post by post_id or piece_id")]
    pub fn get_performance(
        &self,
        #[tool(param)] post_id: Option<i64>,
        #[tool(param)] piece_id: Option<String>,
    ) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;

        let (sql, param) = match (post_id, piece_id) {
            (Some(id), _) => (
                "SELECT COALESCE(m.views,0), COALESCE(m.likes,0), COALESCE(m.comments,0), \
                 COALESCE(m.shares,0), COALESCE(m.watch_time_sec,0) \
                 FROM posts p LEFT JOIN metrics m ON p.id = m.post_id \
                 WHERE p.id = ?".to_string(),
                id.to_string(),
            ),
            (None, Some(pid)) => (
                "SELECT COALESCE(m.views,0), COALESCE(m.likes,0), COALESCE(m.comments,0), \
                 COALESCE(m.shares,0), COALESCE(m.watch_time_sec,0) \
                 FROM posts p LEFT JOIN metrics m ON p.id = m.post_id \
                 WHERE p.piece_id = ? \
                 ORDER BY p.posted_at DESC LIMIT 1".to_string(),
                pid,
            ),
            (None, None) => return Err("post_id or piece_id required".to_string()),
        };

        let row = conn
            .query_row(&sql, params![param], |row| {
                Ok(PerformanceResponse {
                    views: row.get(0)?,
                    likes: row.get(1)?,
                    comments: row.get(2)?,
                    shares: row.get(3)?,
                    watch_time_sec: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;

        serde_json::to_string(&row).map_err(|e| e.to_string())
    }

    #[tool(name = "query_what_worked", description = "Aggregate performance grouped by format, platform or product_sku")]
    pub fn query_what_worked(
        &self,
        #[tool(param)] group_by: String,
        #[tool(param)] date_from: Option<String>,
        #[tool(param)] date_to: Option<String>,
    ) -> Result<String, String> {
        let column = match group_by.as_str() {
            "format" => "format",
            "platform" => "platform",
            "product_sku" => "product_sku",
            _ => return Err("group_by must be format, platform or product_sku".to_string()),
        };

        let mut sql = format!(
            "SELECT p.{}, AVG(COALESCE(m.views,0)), AVG(COALESCE(m.likes,0)), \
             AVG(COALESCE(m.watch_time_sec,0)), COUNT(*) \
             FROM posts p LEFT JOIN metrics m ON p.id = m.post_id WHERE 1=1",
            column
        );
        let mut params_list: Vec<String> = Vec::new();
        if let Some(from) = date_from {
            sql.push_str(" AND p.posted_at >= ?");
            params_list.push(from);
        }
        if let Some(to) = date_to {
            sql.push_str(" AND p.posted_at <= ?");
            params_list.push(to);
        }
        sql.push_str(&format!(" GROUP BY p.{} ORDER BY AVG(COALESCE(m.views,0)) DESC", column));

        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_list.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(WhatWorkedRow {
                    key: row.get(0)?,
                    avg_views: row.get(1)?,
                    avg_likes: row.get(2)?,
                    avg_retention: row.get(3)?,
                    count: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        serde_json::to_string(&rows).map_err(|e| e.to_string())
    }

    #[tool(name = "get_hooks_leaderboard", description = "Return top hooks by average retention")]
    pub fn get_hooks_leaderboard(
        &self,
        #[tool(param)] limit: Option<i64>,
        #[tool(param)] format: Option<String>,
    ) -> Result<String, String> {
        let limit = limit.unwrap_or(10);
        let mut sql =
            "SELECT hook_text, avg_retention, uses, last_used_at FROM hooks WHERE 1=1".to_string();
        let mut params_list: Vec<String> = Vec::new();
        if let Some(fmt) = format {
            sql.push_str(" AND format = ?");
            params_list.push(fmt);
        }
        sql.push_str(" ORDER BY avg_retention DESC LIMIT ?");

        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_list
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .chain(std::iter::once(&limit as &dyn rusqlite::ToSql))
            .collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(HookLeaderboardRow {
                    hook_text: row.get(0)?,
                    avg_retention: row.get(1)?,
                    uses: row.get(2)?,
                    last_used_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        serde_json::to_string(&rows).map_err(|e| e.to_string())
    }
}

impl ServerHandler for LedgerServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-ledger".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Performance ledger MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_server() -> LedgerServer {
        let db = db::open_memory().expect("open memory db");
        LedgerServer {
            db: Arc::new(Mutex::new(db)),
        }
    }

    fn seed_metrics(server: &LedgerServer, post_id: i64, views: i64, likes: i64, watch: i64) {
        let conn = server.db.lock().unwrap();
        conn.execute(
            "UPDATE metrics SET views = ?1, likes = ?2, comments = 0, shares = 0, watch_time_sec = ?3 WHERE post_id = ?4",
            params![views, likes, watch, post_id],
        )
        .unwrap();
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-ledger"));
    }

    #[test]
    fn test_record_post_and_get_performance() {
        let s = test_server();
        let out = s
            .record_post(
                "p1".to_string(),
                "MA5".to_string(),
                "demo".to_string(),
                "youtube".to_string(),
                None,
                None,
                Some("Hook A".to_string()),
            )
            .unwrap();
        let resp: RecordPostResponse = serde_json::from_str(&out).unwrap();
        assert_eq!(resp.post_id, 1);

        seed_metrics(&s, resp.post_id, 1000, 50, 120);

        let perf = s.get_performance(Some(resp.post_id), None).unwrap();
        let perf: PerformanceResponse = serde_json::from_str(&perf).unwrap();
        assert_eq!(perf.views, 1000);
        assert_eq!(perf.likes, 50);
        assert_eq!(perf.watch_time_sec, 120);
    }

    #[test]
    fn test_query_what_worked() {
        let s = test_server();
        for (fmt, views, likes, watch) in [
            ("demo", 1000, 50, 120),
            ("ugc", 500, 20, 60),
            ("demo", 2000, 100, 240),
        ] {
            let out = s
                .record_post(
                    format!("p-{}", fmt),
                    "MA5".to_string(),
                    fmt.to_string(),
                    "youtube".to_string(),
                    None,
                    None,
                    None,
                )
                .unwrap();
            let resp: RecordPostResponse = serde_json::from_str(&out).unwrap();
            seed_metrics(&s, resp.post_id, views, likes, watch);
        }

        let out = s.query_what_worked("format".to_string(), None, None).unwrap();
        let rows: Vec<WhatWorkedRow> = serde_json::from_str(&out).unwrap();
        assert_eq!(rows.len(), 2);
        let demo = rows.iter().find(|r| r.key == "demo").unwrap();
        assert_eq!(demo.count, 2);
        assert!(demo.avg_views > 1400.0 && demo.avg_views < 1600.0);
    }

    #[test]
    fn test_hooks_leaderboard() {
        let s = test_server();
        let out = s
            .record_post(
                "p1".to_string(),
                "MA5".to_string(),
                "demo".to_string(),
                "youtube".to_string(),
                None,
                None,
                Some("Top hook".to_string()),
            )
            .unwrap();
        let resp: RecordPostResponse = serde_json::from_str(&out).unwrap();
        seed_metrics(&s, resp.post_id, 1000, 50, 120);

        let out = s.get_hooks_leaderboard(Some(5), None).unwrap();
        let rows: Vec<HookLeaderboardRow> = serde_json::from_str(&out).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].hook_text, "Top hook");
    }
}
