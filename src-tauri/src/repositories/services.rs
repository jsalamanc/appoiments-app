use rusqlite::{params, Connection};

use crate::commands::services::{ListServicesResult, ServiceInput};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::service::Service;
use crate::util::catalog::{normalize_status, pick_count, status_counts};
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

pub fn list(
    conn: &mut Connection,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListServicesResult> {
    let status = normalize_status(status);
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);

    let counts = status_counts(conn, "services")?;
    let total = pick_count(&counts, &status);
    let total_pages = get_total_pages(total, page_size);
    let safe_page = page.min(total_pages);
    let (limit, offset) = get_range(safe_page, page_size);

    let items = if status == "all" {
        let mut stmt = conn
            .prepare("SELECT * FROM services ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")?;
        let rows = stmt.query_map(params![limit, offset], Service::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT * FROM services WHERE status = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![status, limit, offset], Service::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    Ok(ListServicesResult {
        items,
        total,
        page: safe_page,
        total_pages,
        page_size,
        status,
        counts,
    })
}

pub fn get(conn: &mut Connection, id: String) -> AppResult<Option<Service>> {
    let result = conn.query_row(
        "SELECT * FROM services WHERE id = ?1",
        params![id],
        Service::from_row,
    );
    match result {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn validate_service_input(input: &ServiceInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("El nombre es obligatorio"));
    }
    if input.price < 0.0 {
        return Err(AppError::validation("El precio no puede ser negativo"));
    }
    if input.duration_minutes <= 0 {
        return Err(AppError::validation("La duración debe ser mayor a 0"));
    }
    if input.status != "draft" && input.status != "published" {
        return Err(AppError::validation("Estado inválido"));
    }
    Ok(())
}

pub fn create(conn: &mut Connection, input: ServiceInput) -> AppResult<Service> {
    validate_service_input(&input)?;

    let id = new_id();
    let now = now_iso();
    let description = input.description.as_deref().map(|d| d.trim().to_string());
    let description = description.filter(|d| !d.is_empty());

    conn.execute(
        "INSERT INTO services
            (id, name, description, price, duration_minutes, photo_path, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            id,
            input.name.trim(),
            description,
            input.price,
            input.duration_minutes,
            input.photo_path,
            input.status,
            now
        ],
    )?;

    let service = conn.query_row(
        "SELECT * FROM services WHERE id = ?1",
        params![id],
        Service::from_row,
    )?;
    Ok(service)
}

pub fn update(conn: &mut Connection, id: String, input: ServiceInput) -> AppResult<Service> {
    validate_service_input(&input)?;

    let description = input.description.as_deref().map(|d| d.trim().to_string());
    let description = description.filter(|d| !d.is_empty());

    conn.execute(
        "UPDATE services SET name = ?1, description = ?2, price = ?3, duration_minutes = ?4,
            photo_path = ?5, status = ?6, updated_at = ?7
         WHERE id = ?8",
        params![
            input.name.trim(),
            description,
            input.price,
            input.duration_minutes,
            input.photo_path,
            input.status,
            now_iso(),
            id
        ],
    )?;

    let service = conn.query_row(
        "SELECT * FROM services WHERE id = ?1",
        params![id],
        Service::from_row,
    )?;
    Ok(service)
}

pub fn delete(conn: &mut Connection, id: String) -> AppResult<()> {
    let affected = conn.execute("DELETE FROM services WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn assert_exists(conn: &mut Connection, service_id: &str) -> AppResult<()> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM services WHERE id = ?1",
        params![service_id],
        |r| r.get(0),
    )?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
