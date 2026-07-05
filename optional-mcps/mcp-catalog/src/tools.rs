use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CatalogServer {
    pub db: Arc<Mutex<Connection>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProductRow {
    sku: String,
    name: String,
    category: String,
    price_vnd: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProductDetail {
    sku: String,
    name: String,
    specs_json: String,
    price_vnd: i64,
    tags: String,
    image_paths: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchRow {
    sku: String,
    name: String,
    category: String,
    price_vnd: i64,
    tags: String,
}

#[tool(tool_box)]
impl CatalogServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-catalog".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "list_products", description = "List products with optional category filter")]
    pub fn list_products(
        &self,
        #[tool(param)] category: Option<String>,
        #[tool(param)] limit: Option<i64>,
    ) -> Result<String, String> {
        let limit = limit.unwrap_or(100);
        let rows: Vec<ProductRow> = {
            let conn = self.db.lock().map_err(|e| e.to_string())?;
            match &category {
                Some(cat) => {
                    let mut stmt = conn
                        .prepare(
                            "SELECT sku, name, category, price_vnd FROM products WHERE category = ? LIMIT ?",
                        )
                        .map_err(|e| e.to_string())?;
                    let rows: Vec<ProductRow> = stmt
                        .query_map([cat.as_str(), &limit.to_string()], |row| {
                            Ok(ProductRow {
                                sku: row.get(0)?,
                                name: row.get(1)?,
                                category: row.get(2)?,
                                price_vnd: row.get(3)?,
                            })
                        })
                        .map_err(|e| e.to_string())?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?;
                    rows
                }
                None => {
                    let mut stmt = conn
                        .prepare("SELECT sku, name, category, price_vnd FROM products LIMIT ?")
                        .map_err(|e| e.to_string())?;
                    let rows: Vec<ProductRow> = stmt
                        .query_map([&limit.to_string()], |row| {
                            Ok(ProductRow {
                                sku: row.get(0)?,
                                name: row.get(1)?,
                                category: row.get(2)?,
                                price_vnd: row.get(3)?,
                            })
                        })
                        .map_err(|e| e.to_string())?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?;
                    rows
                }
            }
        };

        serde_json::to_string(&rows).map_err(|e| e.to_string())
    }

    #[tool(name = "get_product_specs", description = "Get full product specs by SKU")]
    pub fn get_product_specs(
        &self,
        #[tool(param)] sku: String,
    ) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let row: ProductDetail = conn
            .query_row(
                "SELECT sku, name, specs_json, price_vnd, tags, image_paths FROM products WHERE sku = ?",
                [&sku],
                |row| {
                    Ok(ProductDetail {
                        sku: row.get(0)?,
                        name: row.get(1)?,
                        specs_json: row.get(2)?,
                        price_vnd: row.get(3)?,
                        tags: row.get(4)?,
                        image_paths: row.get(5)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;

        serde_json::to_string(&row).map_err(|e| e.to_string())
    }

    #[tool(name = "search_catalog", description = "Search products by name, tags or category")]
    pub fn search_catalog(
        &self,
        #[tool(param)] query: String,
        #[tool(param)] limit: Option<i64>,
    ) -> Result<String, String> {
        let limit = limit.unwrap_or(100);
        let pattern = format!("%{}%", query);
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT sku, name, category, price_vnd, tags FROM products \
                 WHERE name LIKE ? OR tags LIKE ? OR category LIKE ? LIMIT ?",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(
                [
                    pattern.as_str(),
                    pattern.as_str(),
                    pattern.as_str(),
                    &limit.to_string(),
                ],
                |row| {
                    Ok(SearchRow {
                        sku: row.get(0)?,
                        name: row.get(1)?,
                        category: row.get(2)?,
                        price_vnd: row.get(3)?,
                        tags: row.get(4)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        serde_json::to_string(&rows).map_err(|e| e.to_string())
    }
}

impl ServerHandler for CatalogServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-catalog".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Product catalog MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_server() -> CatalogServer {
        let db = db::open_memory().expect("open memory db");
        CatalogServer {
            db: Arc::new(Mutex::new(db)),
        }
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-catalog"));
    }

    #[test]
    fn test_list_products_all() {
        let s = test_server();
        let out = s.list_products(None, None).unwrap();
        let rows: Vec<ProductRow> = serde_json::from_str(&out).unwrap();
        assert_eq!(rows.len(), 9);
    }

    #[test]
    fn test_list_products_by_category() {
        let s = test_server();
        let out = s.list_products(Some("audio".to_string()), None).unwrap();
        let rows: Vec<ProductRow> = serde_json::from_str(&out).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.category == "audio"));
    }

    #[test]
    fn test_get_product_specs() {
        let s = test_server();
        let out = s.get_product_specs("MA5".to_string()).unwrap();
        let detail: ProductDetail = serde_json::from_str(&out).unwrap();
        assert_eq!(detail.sku, "MA5");
        assert!(detail.name.contains("LED Matrix"));
    }

    #[test]
    fn test_search_catalog() {
        let s = test_server();
        let out = s.search_catalog("mic".to_string(), None).unwrap();
        let rows: Vec<SearchRow> = serde_json::from_str(&out).unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.sku == "UHF" || r.sku == "mic-deo-tai"));
    }
}
