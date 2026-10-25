//! Migración automática de datos Local → PostgreSQL (issue #11), ofrecida en
//! Ajustes al cambiar de modo de almacenamiento. Copia todo lo que ya
//! trabajaba en SQLite hacia una base PostgreSQL vacía, preservando `id`s
//! (integridad referencial) y timestamps originales — no es un export
//! genérico ni pasa por los comandos típicos de creación (que generan `id`s y
//! timestamps nuevos y re-validan reglas de negocio pensadas para altas, no
//! para una copia histórica).
//!
//! Se separa en dos fases porque `rusqlite` es síncrono y, por la misma
//! convención que ya sigue [`crate::app_db::AppDb::with_sqlite`], nunca debe
//! mezclarse con puntos `.await`:
//! - [`read_local_snapshot`]: lectura síncrona de SQLite, pensada para correr
//!   dentro de `AppDb::with_sqlite` (un hilo bloqueante).
//! - [`write_snapshot_to_postgres`]: una sola transacción async en Postgres.
//!
//! Deliberadamente **no** migra la tabla `sessions` (tokens de sesión
//! vigentes, sin sentido en una base nueva) y **sí** migra `users.password_hash`
//! tal cual, sin volver a hashearla — a diferencia de `crate::export`, esto es
//! una copia directa de base a base, nunca un archivo que pueda leer un
//! tercero.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};
use crate::models::appointment::Appointment;
use crate::models::customer::Customer;
use crate::models::employee::Employee;
use crate::models::inventory::InventoryMovement;
use crate::models::package::{Package, PackageItem};
use crate::models::product::Product;
use crate::models::sale::{Sale, SaleItem};
use crate::models::service::Service;

struct BusinessProfileRow {
    name: String,
    module_employees: bool,
    module_appointments: bool,
    module_services: bool,
    module_packages: bool,
    module_reports: bool,
    created_at: String,
    updated_at: String,
}

/// A diferencia de `UserPublic` (que a propósito no expone `password_hash`),
/// esta fila cruda es la que de verdad hace falta migrar.
struct RawUserRow {
    id: String,
    name: String,
    username: String,
    password_hash: String,
    role: String,
    active: bool,
    created_at: String,
    updated_at: String,
}

pub struct LocalSnapshot {
    business_profile: Option<BusinessProfileRow>,
    users: Vec<RawUserRow>,
    customers: Vec<Customer>,
    products: Vec<Product>,
    services: Vec<Service>,
    packages: Vec<Package>,
    package_items: Vec<PackageItem>,
    employees: Vec<Employee>,
    appointments: Vec<Appointment>,
    sales: Vec<Sale>,
    sale_items: Vec<SaleItem>,
    inventory_movements: Vec<InventoryMovement>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationSummary {
    pub business_profile: u8,
    pub users: usize,
    pub customers: usize,
    pub employees: usize,
    pub products: usize,
    pub services: usize,
    pub packages: usize,
    pub package_items: usize,
    pub appointments: usize,
    pub sales: usize,
    pub sale_items: usize,
    pub inventory_movements: usize,
}

fn read_business_profile(conn: &Connection) -> AppResult<Option<BusinessProfileRow>> {
    conn.query_row(
        "SELECT name, module_employees, module_appointments, module_services,
                module_packages, module_reports, created_at, updated_at
         FROM business_profile WHERE id = 1",
        [],
        |row| {
            Ok(BusinessProfileRow {
                name: row.get("name")?,
                module_employees: row.get::<_, i64>("module_employees")? != 0,
                module_appointments: row.get::<_, i64>("module_appointments")? != 0,
                module_services: row.get::<_, i64>("module_services")? != 0,
                module_packages: row.get::<_, i64>("module_packages")? != 0,
                module_reports: row.get::<_, i64>("module_reports")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    )
    .optional()
    .map_err(AppError::from)
}

fn read_users(conn: &Connection) -> AppResult<Vec<RawUserRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, username, password_hash, role, active, created_at, updated_at
         FROM users ORDER BY created_at ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RawUserRow {
                id: row.get("id")?,
                name: row.get("name")?,
                username: row.get("username")?,
                password_hash: row.get("password_hash")?,
                role: row.get("role")?,
                active: row.get::<_, i64>("active")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// `table` siempre es un literal fijo pasado por este mismo archivo, nunca
/// por el usuario.
fn read_all<T>(
    conn: &Connection,
    table: &str,
    mapper: fn(&rusqlite::Row) -> rusqlite::Result<T>,
) -> AppResult<Vec<T>> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM {table} ORDER BY created_at ASC"))?;
    let rows = stmt
        .query_map([], mapper)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Lee una foto completa de los datos locales a migrar. Pensada para correr
/// dentro de `AppDb::with_sqlite` (hilo bloqueante) — no hace ninguna llamada
/// async.
pub fn read_local_snapshot(conn: &mut Connection) -> AppResult<LocalSnapshot> {
    Ok(LocalSnapshot {
        business_profile: read_business_profile(conn)?,
        users: read_users(conn)?,
        customers: read_all(conn, "customers", Customer::from_row)?,
        products: read_all(conn, "products", Product::from_row)?,
        services: read_all(conn, "services", Service::from_row)?,
        packages: read_all(conn, "packages", Package::from_row)?,
        package_items: read_all(conn, "package_items", PackageItem::from_row)?,
        employees: read_all(conn, "employees", Employee::from_row)?,
        appointments: read_all(conn, "appointments", Appointment::from_row)?,
        sales: read_all(conn, "sales", Sale::from_row)?,
        sale_items: read_all(conn, "sale_items", SaleItem::from_row)?,
        inventory_movements: read_all(conn, "inventory_movements", InventoryMovement::from_row)?,
    })
}

/// Las columnas de timestamp locales son `TEXT` RFC3339; para bindearlas
/// contra una columna `TIMESTAMPTZ` hace falta parsearlas primero a
/// `OffsetDateTime` — nunca compararlas/castearlas como texto (ver el bug ya
/// corregido en `repositories::sales::postgres::parse_period` por esta misma
/// razón: el cast a texto por defecto de Postgres para `timestamptz` no es
/// RFC3339).
fn parse_ts(value: &str) -> AppResult<time::OffsetDateTime> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map_err(|e| AppError::Internal(format!("timestamp inválido en los datos locales: {e}")))
}

/// Inserta la foto de datos locales en Postgres dentro de una sola
/// transacción: si algo falla, no queda ninguna fila a medio migrar. Rechaza
/// de entrada si el destino ya tiene datos de negocio — esto es una
/// migración de una sola vez hacia una base vacía, no una fusión.
pub async fn write_snapshot_to_postgres(
    pg: &PgPool,
    snapshot: LocalSnapshot,
) -> AppResult<MigrationSummary> {
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pg)
        .await?;
    let customers_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM customers")
        .fetch_one(pg)
        .await?;
    if users_count > 0 || customers_count > 0 {
        return Err(AppError::validation(
            "La base de datos de destino no está vacía: ya tiene usuarios o clientes. \
             La migración automática solo está disponible hacia una base PostgreSQL \
             completamente vacía.",
        ));
    }

    let mut tx = pg.begin().await?;

    if let Some(profile) = &snapshot.business_profile {
        sqlx::query(
            "INSERT INTO business_profile
                (id, name, module_employees, module_appointments, module_services,
                 module_packages, module_reports, created_at, updated_at)
             VALUES (1, $1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&profile.name)
        .bind(profile.module_employees)
        .bind(profile.module_appointments)
        .bind(profile.module_services)
        .bind(profile.module_packages)
        .bind(profile.module_reports)
        .bind(parse_ts(&profile.created_at)?)
        .bind(parse_ts(&profile.updated_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for user in &snapshot.users {
        sqlx::query(
            "INSERT INTO users
                (id, name, username, password_hash, role, active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&user.id)
        .bind(&user.name)
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(user.active)
        .bind(parse_ts(&user.created_at)?)
        .bind(parse_ts(&user.updated_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for c in &snapshot.customers {
        sqlx::query(
            "INSERT INTO customers (id, name, phone, email, document, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&c.id)
        .bind(&c.name)
        .bind(&c.phone)
        .bind(&c.email)
        .bind(&c.document)
        .bind(parse_ts(&c.created_at)?)
        .bind(parse_ts(&c.updated_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for p in &snapshot.products {
        sqlx::query(
            "INSERT INTO products
                (id, name, price, photo_path, status, stock, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&p.id)
        .bind(&p.name)
        .bind(p.price)
        .bind(&p.photo_path)
        .bind(&p.status)
        .bind(p.stock)
        .bind(parse_ts(&p.created_at)?)
        .bind(parse_ts(&p.updated_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for s in &snapshot.services {
        sqlx::query(
            "INSERT INTO services
                (id, name, description, price, duration_minutes, photo_path, status,
                 created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&s.id)
        .bind(&s.name)
        .bind(&s.description)
        .bind(s.price)
        .bind(s.duration_minutes)
        .bind(&s.photo_path)
        .bind(&s.status)
        .bind(parse_ts(&s.created_at)?)
        .bind(parse_ts(&s.updated_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for pk in &snapshot.packages {
        sqlx::query(
            "INSERT INTO packages
                (id, name, description, price, photo_path, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&pk.id)
        .bind(&pk.name)
        .bind(&pk.description)
        .bind(pk.price)
        .bind(&pk.photo_path)
        .bind(&pk.status)
        .bind(parse_ts(&pk.created_at)?)
        .bind(parse_ts(&pk.updated_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for e in &snapshot.employees {
        sqlx::query(
            "INSERT INTO employees (id, name, color, active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&e.id)
        .bind(&e.name)
        .bind(&e.color)
        .bind(e.active)
        .bind(parse_ts(&e.created_at)?)
        .bind(parse_ts(&e.updated_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for pi in &snapshot.package_items {
        sqlx::query(
            "INSERT INTO package_items
                (id, package_id, item_type, product_id, service_id, quantity, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&pi.id)
        .bind(&pi.package_id)
        .bind(&pi.item_type)
        .bind(&pi.product_id)
        .bind(&pi.service_id)
        .bind(pi.quantity)
        .bind(parse_ts(&pi.created_at)?)
        .execute(&mut *tx)
        .await?;
    }

    // `sale_id` se fuerza a NULL acá (literal en el SQL, no bindeado): el
    // valor original se restaura más abajo, una vez que `sales` ya existe.
    for a in &snapshot.appointments {
        sqlx::query(
            "INSERT INTO appointments
                (id, employee_id, customer_id, service_id, service_name_snapshot,
                 starts_at, ends_at, status, notes, sale_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, $10, $11)",
        )
        .bind(&a.id)
        .bind(&a.employee_id)
        .bind(&a.customer_id)
        .bind(&a.service_id)
        .bind(&a.service_name_snapshot)
        .bind(parse_ts(&a.starts_at)?)
        .bind(parse_ts(&a.ends_at)?)
        .bind(&a.status)
        .bind(&a.notes)
        .bind(parse_ts(&a.created_at)?)
        .bind(parse_ts(&a.updated_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for s in &snapshot.sales {
        sqlx::query(
            "INSERT INTO sales
                (id, customer_id, appointment_id, payment_method, total, sold_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&s.id)
        .bind(&s.customer_id)
        .bind(&s.appointment_id)
        .bind(&s.payment_method)
        .bind(s.total)
        .bind(parse_ts(&s.sold_at)?)
        .bind(parse_ts(&s.created_at)?)
        .execute(&mut *tx)
        .await?;
    }

    // Pasada de parche: restaura `sale_id` en las citas que lo tenían,
    // ahora que la venta referenciada ya existe. SQL crudo a propósito —
    // `repositories::sales::postgres::link_to_appointment` no sirve acá
    // porque además pisaría `updated_at` con `now()`.
    for a in &snapshot.appointments {
        if let Some(sale_id) = &a.sale_id {
            sqlx::query("UPDATE appointments SET sale_id = $1 WHERE id = $2")
                .bind(sale_id)
                .bind(&a.id)
                .execute(&mut *tx)
                .await?;
        }
    }

    for si in &snapshot.sale_items {
        sqlx::query(
            "INSERT INTO sale_items
                (id, sale_id, item_type, product_id, service_id, package_id,
                 name_snapshot, unit_price, quantity, line_total, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&si.id)
        .bind(&si.sale_id)
        .bind(&si.item_type)
        .bind(&si.product_id)
        .bind(&si.service_id)
        .bind(&si.package_id)
        .bind(&si.name_snapshot)
        .bind(si.unit_price)
        .bind(si.quantity)
        .bind(si.line_total)
        .bind(parse_ts(&si.created_at)?)
        .execute(&mut *tx)
        .await?;
    }

    for im in &snapshot.inventory_movements {
        sqlx::query(
            "INSERT INTO inventory_movements
                (id, product_id, movement_type, quantity_delta, stock_after, sale_id, note,
                 created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&im.id)
        .bind(&im.product_id)
        .bind(&im.movement_type)
        .bind(im.quantity_delta)
        .bind(im.stock_after)
        .bind(&im.sale_id)
        .bind(&im.note)
        .bind(parse_ts(&im.created_at)?)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(MigrationSummary {
        business_profile: if snapshot.business_profile.is_some() { 1 } else { 0 },
        users: snapshot.users.len(),
        customers: snapshot.customers.len(),
        employees: snapshot.employees.len(),
        products: snapshot.products.len(),
        services: snapshot.services.len(),
        packages: snapshot.packages.len(),
        package_items: snapshot.package_items.len(),
        appointments: snapshot.appointments.len(),
        sales: snapshot.sales.len(),
        sale_items: snapshot.sale_items.len(),
        inventory_movements: snapshot.inventory_movements.len(),
    })
}

/// Punto de entrada usado por `storage_switch_mode`: lee de SQLite (vía
/// `AppDb::with_sqlite`, en un hilo bloqueante) y escribe en Postgres en una
/// sola transacción.
pub async fn migrate_local_to_postgres(
    db: &crate::app_db::AppDb,
    pg: &PgPool,
) -> AppResult<MigrationSummary> {
    let snapshot = db.with_sqlite(read_local_snapshot).await?;
    write_snapshot_to_postgres(pg, snapshot).await
}
