use rusqlite::{Connection, Result};
use std::path::Path;

const SCHEMA_SQL: &str = include_str!("../schema.sql");
const SEED_SQL: &str = include_str!("../seed.sql");

pub fn open_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = Connection::open(path)?;
    init(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_SQL)?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0))?;
    if count == 0 {
        conn.execute_batch(SEED_SQL)?;
    }
    Ok(())
}
