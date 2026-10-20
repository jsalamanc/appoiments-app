use rusqlite::Connection;
use serde::Serialize;

use crate::error::AppResult;

/// Conteos por estado para una tabla de catálogo (`products`, `services`,
/// `packages`). Usado por los `list_*` para alimentar los filtros de
/// Todos/Publicados/Borradores en el frontend.
#[derive(Debug, Serialize)]
pub struct CatalogCounts {
    pub all: i64,
    pub published: i64,
    pub draft: i64,
}

/// `table` es siempre un literal fijo pasado por el código (nunca viene del
/// usuario), así que interpolarlo en el SQL es seguro.
pub fn status_counts(conn: &Connection, table: &str) -> AppResult<CatalogCounts> {
    let all: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?;
    let published: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE status = 'published'"),
        [],
        |r| r.get(0),
    )?;
    let draft: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE status = 'draft'"),
        [],
        |r| r.get(0),
    )?;
    Ok(CatalogCounts {
        all,
        published,
        draft,
    })
}

/// Normaliza el filtro de estado recibido del frontend a "all" | "draft" | "published".
pub fn normalize_status(status: Option<String>) -> String {
    match status.as_deref() {
        Some("draft") => "draft".to_string(),
        Some("published") => "published".to_string(),
        _ => "all".to_string(),
    }
}

pub fn pick_count(counts: &CatalogCounts, status: &str) -> i64 {
    match status {
        "published" => counts.published,
        "draft" => counts.draft,
        _ => counts.all,
    }
}
