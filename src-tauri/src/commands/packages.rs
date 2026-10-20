use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::package::Package;
use crate::state::AppState;
use crate::util::catalog::{normalize_status, pick_count, status_counts, CatalogCounts};
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};
use crate::util::photos::import_photo;

#[derive(Debug, Serialize)]
pub struct ProductRef {
    pub id: String,
    pub name: String,
    pub price: f64,
}

#[derive(Debug, Serialize)]
pub struct ServiceRef {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub duration_minutes: i64,
}

#[derive(Debug, Serialize)]
pub struct PackageItemWithRefs {
    pub id: String,
    pub package_id: String,
    pub item_type: String,
    pub product_id: Option<String>,
    pub service_id: Option<String>,
    pub quantity: i64,
    pub created_at: String,
    pub products: Option<ProductRef>,
    pub services: Option<ServiceRef>,
}

#[derive(Debug, Serialize)]
pub struct PackageWithItems {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub photo_path: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub package_items: Vec<PackageItemWithRefs>,
}

fn with_items(pkg: Package, package_items: Vec<PackageItemWithRefs>) -> PackageWithItems {
    PackageWithItems {
        id: pkg.id,
        name: pkg.name,
        description: pkg.description,
        price: pkg.price,
        photo_path: pkg.photo_path,
        status: pkg.status,
        created_at: pkg.created_at,
        updated_at: pkg.updated_at,
        package_items,
    }
}

fn load_package_items(conn: &Connection, package_id: &str) -> AppResult<Vec<PackageItemWithRefs>> {
    let mut stmt = conn.prepare(
        "SELECT pi.*, p.name as p_name, p.price as p_price,
                s.name as s_name, s.price as s_price, s.duration_minutes as s_duration
         FROM package_items pi
         LEFT JOIN products p ON p.id = pi.product_id
         LEFT JOIN services s ON s.id = pi.service_id
         WHERE pi.package_id = ?1
         ORDER BY pi.created_at ASC",
    )?;
    let items = stmt
        .query_map(params![package_id], |row| {
            let product_id: Option<String> = row.get("product_id")?;
            let service_id: Option<String> = row.get("service_id")?;

            let products = match &product_id {
                Some(pid) => Some(ProductRef {
                    id: pid.clone(),
                    name: row.get("p_name")?,
                    price: row.get("p_price")?,
                }),
                None => None,
            };
            let services = match &service_id {
                Some(sid) => Some(ServiceRef {
                    id: sid.clone(),
                    name: row.get("s_name")?,
                    price: row.get("s_price")?,
                    duration_minutes: row.get("s_duration")?,
                }),
                None => None,
            };

            Ok(PackageItemWithRefs {
                id: row.get("id")?,
                package_id: row.get("package_id")?,
                item_type: row.get("item_type")?,
                product_id,
                service_id,
                quantity: row.get("quantity")?,
                created_at: row.get("created_at")?,
                products,
                services,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

#[derive(Debug, Serialize)]
pub struct ListPackagesResult {
    pub items: Vec<PackageWithItems>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
    pub status: String,
    pub counts: CatalogCounts,
}

#[tauri::command]
pub fn list_packages(
    state: State<AppState>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListPackagesResult> {
    let conn = state.db.lock().unwrap();
    let status = normalize_status(status);
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);

    let counts = status_counts(&conn, "packages")?;
    let total = pick_count(&counts, &status);
    let total_pages = get_total_pages(total, page_size);
    let safe_page = page.min(total_pages);
    let (limit, offset) = get_range(safe_page, page_size);

    let packages: Vec<Package> = if status == "all" {
        let mut stmt = conn
            .prepare("SELECT * FROM packages ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")?;
        let rows = stmt.query_map(params![limit, offset], Package::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT * FROM packages WHERE status = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![status, limit, offset], Package::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut items = Vec::with_capacity(packages.len());
    for pkg in packages {
        let pkg_items = load_package_items(&conn, &pkg.id)?;
        items.push(with_items(pkg, pkg_items));
    }

    Ok(ListPackagesResult {
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
pub fn get_package(state: State<AppState>, id: String) -> AppResult<Option<PackageWithItems>> {
    let conn = state.db.lock().unwrap();
    let pkg = match conn.query_row(
        "SELECT * FROM packages WHERE id = ?1",
        params![id],
        Package::from_row,
    ) {
        Ok(p) => p,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let items = load_package_items(&conn, &pkg.id)?;
    Ok(Some(with_items(pkg, items)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageItemInput {
    pub item_type: String,
    pub item_id: String,
    pub quantity: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInput {
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub photo_path: Option<String>,
    pub status: String,
    pub items: Vec<PackageItemInput>,
}

fn validate_package_input(input: &PackageInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("El nombre es obligatorio"));
    }
    if input.price < 0.0 {
        return Err(AppError::validation("El precio no puede ser negativo"));
    }
    if input.status != "draft" && input.status != "published" {
        return Err(AppError::validation("Estado inválido"));
    }
    if input.items.is_empty() {
        return Err(AppError::validation(
            "Agrega al menos un producto o servicio",
        ));
    }
    for item in &input.items {
        if item.item_type != "product" && item.item_type != "service" {
            return Err(AppError::validation("Tipo de ítem inválido"));
        }
        if item.item_id.trim().is_empty() {
            return Err(AppError::validation("Selecciona un ítem"));
        }
        if item.quantity < 1 {
            return Err(AppError::validation("Cantidad mínima 1"));
        }
    }
    Ok(())
}

fn insert_items(
    tx: &rusqlite::Transaction,
    package_id: &str,
    items: &[PackageItemInput],
) -> AppResult<()> {
    let now = now_iso();
    for item in items {
        let (product_id, service_id) = if item.item_type == "product" {
            (Some(item.item_id.clone()), None)
        } else {
            (None, Some(item.item_id.clone()))
        };
        tx.execute(
            "INSERT INTO package_items
                (id, package_id, item_type, product_id, service_id, quantity, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id(),
                package_id,
                item.item_type,
                product_id,
                service_id,
                item.quantity,
                now
            ],
        )?;
    }
    Ok(())
}

#[tauri::command]
pub fn create_package(
    state: State<AppState>,
    input: PackageInput,
) -> AppResult<PackageWithItems> {
    validate_package_input(&input)?;

    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let id = new_id();
    let now = now_iso();
    let description = input.description.as_deref().map(|d| d.trim().to_string());
    let description = description.filter(|d| !d.is_empty());

    tx.execute(
        "INSERT INTO packages (id, name, description, price, photo_path, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            id,
            input.name.trim(),
            description,
            input.price,
            input.photo_path,
            input.status,
            now
        ],
    )?;

    insert_items(&tx, &id, &input.items)?;

    let pkg = tx.query_row(
        "SELECT * FROM packages WHERE id = ?1",
        params![id],
        Package::from_row,
    )?;
    let items = load_package_items(&tx, &id)?;

    tx.commit()?;
    Ok(with_items(pkg, items))
}

#[tauri::command]
pub fn update_package(
    state: State<AppState>,
    id: String,
    input: PackageInput,
) -> AppResult<PackageWithItems> {
    validate_package_input(&input)?;

    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let description = input.description.as_deref().map(|d| d.trim().to_string());
    let description = description.filter(|d| !d.is_empty());

    let affected = tx.execute(
        "UPDATE packages SET name = ?1, description = ?2, price = ?3, photo_path = ?4,
            status = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            input.name.trim(),
            description,
            input.price,
            input.photo_path,
            input.status,
            now_iso(),
            id
        ],
    )?;
    if affected == 0 {
        return Err(AppError::NotFound);
    }

    tx.execute(
        "DELETE FROM package_items WHERE package_id = ?1",
        params![id],
    )?;
    insert_items(&tx, &id, &input.items)?;

    let pkg = tx.query_row(
        "SELECT * FROM packages WHERE id = ?1",
        params![id],
        Package::from_row,
    )?;
    let items = load_package_items(&tx, &id)?;

    tx.commit()?;
    Ok(with_items(pkg, items))
}

#[tauri::command]
pub fn delete_package(state: State<AppState>, id: String) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    let affected = conn.execute("DELETE FROM packages WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

#[tauri::command]
pub fn import_package_photo(
    app: AppHandle,
    state: State<AppState>,
    package_id: String,
    source_file_path: String,
) -> AppResult<String> {
    {
        let conn = state.db.lock().unwrap();
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM packages WHERE id = ?1",
            params![package_id],
            |r| r.get(0),
        )?;
        if exists == 0 {
            return Err(AppError::NotFound);
        }
    }

    let data_dir = app.path().app_data_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}"))
    })?;

    import_photo(&data_dir, "packages", &source_file_path)
}
