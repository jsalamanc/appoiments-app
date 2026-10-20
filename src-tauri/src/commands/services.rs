use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::service::Service;
use crate::state::AppState;
use crate::util::catalog::{normalize_status, pick_count, status_counts, CatalogCounts};
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};
use crate::util::photos::import_photo;

#[derive(Debug, Serialize)]
pub struct ListServicesResult {
    pub items: Vec<Service>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
    pub status: String,
    pub counts: CatalogCounts,
}

#[tauri::command]
pub fn list_services(
    state: State<AppState>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListServicesResult> {
    let conn = state.db.lock().unwrap();
    let status = normalize_status(status);
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);

    let counts = status_counts(&conn, "services")?;
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

#[tauri::command]
pub fn get_service(state: State<AppState>, id: String) -> AppResult<Option<Service>> {
    let conn = state.db.lock().unwrap();
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInput {
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub duration_minutes: i64,
    pub photo_path: Option<String>,
    pub status: String,
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

#[tauri::command]
pub fn create_service(state: State<AppState>, input: ServiceInput) -> AppResult<Service> {
    validate_service_input(&input)?;

    let conn = state.db.lock().unwrap();
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

#[tauri::command]
pub fn update_service(
    state: State<AppState>,
    id: String,
    input: ServiceInput,
) -> AppResult<Service> {
    validate_service_input(&input)?;

    let conn = state.db.lock().unwrap();
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

#[tauri::command]
pub fn delete_service(state: State<AppState>, id: String) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    let affected = conn.execute("DELETE FROM services WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

#[tauri::command]
pub fn import_service_photo(
    app: AppHandle,
    state: State<AppState>,
    service_id: String,
    source_file_path: String,
) -> AppResult<String> {
    {
        let conn = state.db.lock().unwrap();
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM services WHERE id = ?1",
            params![service_id],
            |r| r.get(0),
        )?;
        if exists == 0 {
            return Err(AppError::NotFound);
        }
    }

    let data_dir = app.path().app_data_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}"))
    })?;

    import_photo(&data_dir, "services", &source_file_path)
}
