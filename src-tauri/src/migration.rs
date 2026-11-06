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

use std::path::Path;

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
use crate::util::bucket::BucketClient;

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
    /// Fotos de productos/servicios/paquetes subidas al bucket (issue #18).
    pub images_migrated: usize,
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

fn parse_ts_opt(value: &Option<String>) -> AppResult<Option<time::OffsetDateTime>> {
    value.as_deref().map(parse_ts).transpose()
}

/// Evento emitido al frontend (`storage-migration-progress`) mientras corre
/// una migración larga, para que el panel de Ajustes pueda cerrarse y dejar
/// seguir navegando (issue pedido por el dueño del negocio: ver un toast tipo
/// Google Drive en vez de bloquear la app) — el conteo real de filas puede
/// tardar varios minutos en una cuenta con mucho historial, al ser una fila
/// por sentencia `INSERT` (ver `write_snapshot_to_postgres`).
#[derive(Debug, Clone, Serialize)]
pub struct MigrationProgressEvent {
    pub done: usize,
    pub total: usize,
}

/// Inserta la foto de datos locales en Postgres dentro de una sola
/// transacción: si algo falla, no queda ninguna fila a medio migrar.
///
/// Por default rechaza si el destino ya tiene datos de negocio — es una
/// migración de una sola vez hacia una base vacía, no una fusión (nunca hay
/// lógica de merge/resolución de conflictos). `replace` cambia ese rechazo
/// por un vaciado completo del destino (mismas tablas que
/// [`clear_local_data`], dialecto Postgres) ANTES de insertar, dentro de la
/// misma transacción que el resto de esta función: si algo falla a mitad de
/// camino, ni el vaciado ni la inserción quedan a medias. Pensado para
/// reconectar a un negocio de Nube/Postgres propio que ya se usó antes (ver
/// `commands::storage::storage_switch_mode`) y que el usuario decide
/// explícitamente reemplazar con lo que tiene en Local ahora.
///
/// `on_progress(done, total)` se llama después de cada fila insertada. Un
/// callback plano en vez de recibir un `AppHandle` directamente, para que
/// este módulo no dependa de Tauri — lo único que lo necesita de verdad es
/// `commands::storage::storage_switch_mode`, que arma el callback cerrando
/// sobre un `AppHandle` clonado para emitir `storage-migration-progress`
/// (`examples/pg_smoke_test.rs`, que corre fuera de una app Tauri, pasa un
/// callback vacío).
pub async fn write_snapshot_to_postgres(
    on_progress: &mut (dyn FnMut(usize, usize) + Send),
    pg: &PgPool,
    snapshot: LocalSnapshot,
    replace: bool,
) -> AppResult<MigrationSummary> {
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pg)
        .await?;
    let customers_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM customers")
        .fetch_one(pg)
        .await?;
    let destination_has_data = users_count > 0 || customers_count > 0;
    if destination_has_data && !replace {
        return Err(AppError::validation(
            "La base de datos de destino no está vacía: ya tiene usuarios o clientes. \
             Marca \"reemplazar\" si quieres vaciarla y reemplazarla con tus datos actuales, \
             o usa una base completamente vacía.",
        ));
    }

    let total = usize::from(snapshot.business_profile.is_some())
        + snapshot.users.len()
        + snapshot.customers.len()
        + snapshot.products.len()
        + snapshot.services.len()
        + snapshot.packages.len()
        + snapshot.package_items.len()
        + snapshot.employees.len()
        + snapshot.appointments.len()
        + snapshot.sales.len()
        + snapshot
            .appointments
            .iter()
            .filter(|a| a.sale_id.is_some())
            .count()
        + snapshot.sale_items.len()
        + snapshot.inventory_movements.len();
    let mut done: usize = 0;
    let mut tick = |done: &mut usize| {
        *done += 1;
        on_progress(*done, total);
    };

    let mut tx = pg.begin().await?;

    if destination_has_data {
        // `replace` ya se confirmó arriba (si no, ya se retornó el error) --
        // dentro de la misma transacción que las inserciones de abajo, para
        // que un fallo a mitad de camino no deje el destino vacío sin los
        // datos nuevos.
        sqlx::query(
            "TRUNCATE TABLE
                inventory_movements, sale_payments, sale_items, sales, appointments,
                package_items, packages, services, products, employee_availability,
                employee_commission_rates, employees, cash_sessions, customers, sessions,
                users, business_profile",
        )
        .execute(&mut *tx)
        .await?;
    }

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
        tick(&mut done);
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
        tick(&mut done);
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
        tick(&mut done);
    }

    for p in &snapshot.products {
        sqlx::query(
            "INSERT INTO products
                (id, name, price, cost, photo_path, status, stock, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&p.id)
        .bind(&p.name)
        .bind(p.price)
        .bind(p.cost)
        .bind(&p.photo_path)
        .bind(&p.status)
        .bind(p.stock)
        .bind(parse_ts(&p.created_at)?)
        .bind(parse_ts(&p.updated_at)?)
        .execute(&mut *tx)
        .await?;
        tick(&mut done);
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
        tick(&mut done);
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
        tick(&mut done);
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
        tick(&mut done);
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
        tick(&mut done);
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
        tick(&mut done);
    }

    for s in &snapshot.sales {
        sqlx::query(
            "INSERT INTO sales
                (id, customer_id, appointment_id, user_id, payment_method, total,
                 discount_amount, sold_at, created_at, voided_at, void_reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&s.id)
        .bind(&s.customer_id)
        .bind(&s.appointment_id)
        .bind(&s.user_id)
        .bind(&s.payment_method)
        .bind(s.total)
        .bind(s.discount_amount)
        .bind(parse_ts(&s.sold_at)?)
        .bind(parse_ts(&s.created_at)?)
        .bind(parse_ts_opt(&s.voided_at)?)
        .bind(&s.void_reason)
        .execute(&mut *tx)
        .await?;
        tick(&mut done);
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
            tick(&mut done);
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
        tick(&mut done);
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
        tick(&mut done);
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
        // Se completa en `migrate_local_to_postgres`, que es quien conoce el
        // resultado de la subida de imágenes al bucket (ver `migrate_images`).
        images_migrated: 0,
    })
}

/// Sube al bucket cada imagen de producto/servicio/paquete referenciada en el
/// snapshot, usando la misma ruta relativa (`photo_path`) como key del
/// objeto — ver la decisión de diseño en `crate::util::photos`. Se corre
/// ANTES de abrir la transacción de Postgres: si alguna subida falla, la
/// migración se aborta completa y no se inserta ninguna fila (mismo
/// principio "todo o nada" que ya sigue `write_snapshot_to_postgres`).
async fn migrate_images(
    data_dir: &Path,
    bucket_config: &crate::bucket_config::BucketConfig,
    bucket: &BucketClient,
    snapshot: &LocalSnapshot,
) -> AppResult<usize> {
    let paths = snapshot
        .products
        .iter()
        .filter_map(|p| p.photo_path.as_deref())
        .chain(snapshot.services.iter().filter_map(|s| s.photo_path.as_deref()))
        .chain(snapshot.packages.iter().filter_map(|pk| pk.photo_path.as_deref()));

    let mut count = 0;
    for photo_path in paths {
        let bytes = std::fs::read(data_dir.join(photo_path)).map_err(|e| {
            AppError::validation(format!(
                "no se pudo leer la imagen local \"{photo_path}\" para migrarla: {e}"
            ))
        })?;
        bucket.put_object(&bucket_config.object_key(photo_path), bytes).await?;
        count += 1;
    }
    Ok(count)
}

/// Punto de entrada usado por `storage_switch_mode`: lee de SQLite (vía
/// `AppDb::with_sqlite`, en un hilo bloqueante), sube las imágenes del
/// catálogo al bucket (issue #18) y escribe los datos en Postgres en una
/// sola transacción.
pub async fn migrate_local_to_postgres(
    on_progress: &mut (dyn FnMut(usize, usize) + Send),
    db: &crate::app_db::AppDb,
    pg: &PgPool,
    data_dir: &Path,
    bucket: Option<(&crate::bucket_config::BucketConfig, &BucketClient)>,
    replace: bool,
) -> AppResult<MigrationSummary> {
    let snapshot = db.with_sqlite(read_local_snapshot).await?;
    // Sin bucket (modo Nube sin configurar uno todavía, ver
    // `commands::storage::storage_switch_mode`) no hay a dónde subir las
    // imágenes -- se migra el resto de los datos igual, dejando
    // `images_migrated` en 0; las fotos locales simplemente no viajan.
    let images_migrated = match bucket {
        Some((bucket_config, bucket_client)) => {
            migrate_images(data_dir, bucket_config, bucket_client, &snapshot).await?
        }
        None => 0,
    };
    let mut summary = write_snapshot_to_postgres(on_progress, pg, snapshot, replace).await?;
    summary.images_migrated = images_migrated;
    Ok(summary)
}

// ---------------------------------------------------------------------------
// PostgreSQL → Local (issue pendiente del roadmap de Nube: "migración de
// vuelta"). Espejo de las funciones de arriba, en la dirección contraria:
// lee una foto completa de Postgres (async) y la escribe en una transacción
// SQLite (síncrona, dentro de `AppDb::with_sqlite`-equivalente a mano, ya que
// acá no hay un `AppDb::Sqlite` todavía armado -- la conexión local se abre
// recién en `migrate_postgres_to_local`).
// ---------------------------------------------------------------------------

fn map_customer_pg(row: &sqlx::postgres::PgRow) -> AppResult<Customer> {
    use sqlx::Row;
    let created_at: time::OffsetDateTime = row.try_get("created_at")?;
    let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
    Ok(Customer {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        phone: row.try_get("phone")?,
        email: row.try_get("email")?,
        document: row.try_get("document")?,
        created_at: crate::db::format_pg_timestamp(created_at),
        updated_at: crate::db::format_pg_timestamp(updated_at),
    })
}

fn map_appointment_pg(row: &sqlx::postgres::PgRow) -> AppResult<Appointment> {
    use sqlx::Row;
    let starts_at: time::OffsetDateTime = row.try_get("starts_at")?;
    let ends_at: time::OffsetDateTime = row.try_get("ends_at")?;
    let created_at: time::OffsetDateTime = row.try_get("created_at")?;
    let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
    Ok(Appointment {
        id: row.try_get("id")?,
        employee_id: row.try_get("employee_id")?,
        customer_id: row.try_get("customer_id")?,
        service_id: row.try_get("service_id")?,
        service_name_snapshot: row.try_get("service_name_snapshot")?,
        starts_at: crate::db::format_pg_timestamp(starts_at),
        ends_at: crate::db::format_pg_timestamp(ends_at),
        status: row.try_get("status")?,
        notes: row.try_get("notes")?,
        sale_id: row.try_get("sale_id")?,
        created_at: crate::db::format_pg_timestamp(created_at),
        updated_at: crate::db::format_pg_timestamp(updated_at),
    })
}

fn map_business_profile_pg(row: &sqlx::postgres::PgRow) -> AppResult<BusinessProfileRow> {
    use sqlx::Row;
    let created_at: time::OffsetDateTime = row.try_get("created_at")?;
    let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
    Ok(BusinessProfileRow {
        name: row.try_get("name")?,
        module_employees: row.try_get("module_employees")?,
        module_appointments: row.try_get("module_appointments")?,
        module_services: row.try_get("module_services")?,
        module_packages: row.try_get("module_packages")?,
        module_reports: row.try_get("module_reports")?,
        created_at: crate::db::format_pg_timestamp(created_at),
        updated_at: crate::db::format_pg_timestamp(updated_at),
    })
}

fn map_user_pg(row: &sqlx::postgres::PgRow) -> AppResult<RawUserRow> {
    use sqlx::Row;
    let created_at: time::OffsetDateTime = row.try_get("created_at")?;
    let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
    Ok(RawUserRow {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        username: row.try_get("username")?,
        password_hash: row.try_get("password_hash")?,
        role: row.try_get("role")?,
        active: row.try_get("active")?,
        created_at: crate::db::format_pg_timestamp(created_at),
        updated_at: crate::db::format_pg_timestamp(updated_at),
    })
}

async fn read_postgres_snapshot(pg: &PgPool) -> AppResult<LocalSnapshot> {
    let business_profile = sqlx::query("SELECT * FROM business_profile WHERE id = 1")
        .fetch_optional(pg)
        .await?
        .as_ref()
        .map(map_business_profile_pg)
        .transpose()?;

    let users = sqlx::query("SELECT * FROM users ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(map_user_pg)
        .collect::<AppResult<Vec<_>>>()?;

    let customers = sqlx::query("SELECT * FROM customers ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(map_customer_pg)
        .collect::<AppResult<Vec<_>>>()?;

    let products = sqlx::query("SELECT * FROM products ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(Product::from_pg_row)
        .collect::<AppResult<Vec<_>>>()?;

    let services = sqlx::query("SELECT * FROM services ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(Service::from_pg_row)
        .collect::<AppResult<Vec<_>>>()?;

    let packages = sqlx::query("SELECT * FROM packages ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(Package::from_pg_row)
        .collect::<AppResult<Vec<_>>>()?;

    let package_items = sqlx::query("SELECT * FROM package_items ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(PackageItem::from_pg_row)
        .collect::<AppResult<Vec<_>>>()?;

    let employees = sqlx::query("SELECT * FROM employees ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(Employee::from_pg_row)
        .collect::<AppResult<Vec<_>>>()?;

    let appointments = sqlx::query("SELECT * FROM appointments ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(map_appointment_pg)
        .collect::<AppResult<Vec<_>>>()?;

    let sales = sqlx::query("SELECT * FROM sales ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(Sale::from_pg_row)
        .collect::<AppResult<Vec<_>>>()?;

    let sale_items = sqlx::query("SELECT * FROM sale_items ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(SaleItem::from_pg_row)
        .collect::<AppResult<Vec<_>>>()?;

    let inventory_movements = sqlx::query("SELECT * FROM inventory_movements ORDER BY created_at ASC")
        .fetch_all(pg)
        .await?
        .iter()
        .map(InventoryMovement::from_pg_row)
        .collect::<AppResult<Vec<_>>>()?;

    Ok(LocalSnapshot {
        business_profile,
        users,
        customers,
        products,
        services,
        packages,
        package_items,
        employees,
        appointments,
        sales,
        sale_items,
        inventory_movements,
    })
}

/// Espejo síncrono de [`write_snapshot_to_postgres`] — mismo principio "todo
/// o nada" en una sola transacción SQLite, mismo rechazo si el destino ya
/// tiene datos (acá, la base local). Los timestamps ya vienen como `String`
/// RFC3339 (convertidos por los `map_*_pg`/`from_pg_row` de arriba), así que
/// se escriben tal cual en las columnas `TEXT` locales, sin volver a
/// parsearlos.
fn write_snapshot_to_local(
    on_progress: &mut (dyn FnMut(usize, usize) + Send),
    conn: &mut Connection,
    snapshot: LocalSnapshot,
    replace: bool,
) -> AppResult<MigrationSummary> {
    let users_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
    let customers_count: i64 = conn.query_row("SELECT COUNT(*) FROM customers", [], |r| r.get(0))?;
    let destination_has_data = users_count > 0 || customers_count > 0;
    if destination_has_data && !replace {
        return Err(AppError::validation(
            "La base de datos local ya tiene datos (usuarios o clientes). \
             Marca \"reemplazar\" si quieres vaciarla y reemplazarla con tus datos actuales, \
             o usa una base local vacía.",
        ));
    }

    let total = usize::from(snapshot.business_profile.is_some())
        + snapshot.users.len()
        + snapshot.customers.len()
        + snapshot.products.len()
        + snapshot.services.len()
        + snapshot.packages.len()
        + snapshot.package_items.len()
        + snapshot.employees.len()
        + snapshot.appointments.len()
        + snapshot.sales.len()
        + snapshot
            .appointments
            .iter()
            .filter(|a| a.sale_id.is_some())
            .count()
        + snapshot.sale_items.len()
        + snapshot.inventory_movements.len();
    let mut done: usize = 0;
    let mut tick = |done: &mut usize| {
        *done += 1;
        on_progress(*done, total);
    };

    let tx = conn.transaction()?;

    if destination_has_data {
        // `replace` ya se confirmó arriba. Reusa `clear_local_data` sobre la
        // misma transacción (`Transaction` implementa `Deref<Target =
        // Connection>`) para que un fallo a mitad de camino no deje la base
        // local vacía sin los datos nuevos tampoco.
        clear_local_data(&tx)?;
    }

    if let Some(profile) = &snapshot.business_profile {
        tx.execute(
            "INSERT INTO business_profile
                (id, name, module_employees, module_appointments, module_services,
                 module_packages, module_reports, created_at, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                profile.name,
                profile.module_employees as i64,
                profile.module_appointments as i64,
                profile.module_services as i64,
                profile.module_packages as i64,
                profile.module_reports as i64,
                profile.created_at,
                profile.updated_at,
            ],
        )?;
        tick(&mut done);
    }

    for user in &snapshot.users {
        tx.execute(
            "INSERT INTO users
                (id, name, username, password_hash, role, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                user.id,
                user.name,
                user.username,
                user.password_hash,
                user.role,
                user.active as i64,
                user.created_at,
                user.updated_at,
            ],
        )?;
        tick(&mut done);
    }

    for c in &snapshot.customers {
        tx.execute(
            "INSERT INTO customers (id, name, phone, email, document, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![c.id, c.name, c.phone, c.email, c.document, c.created_at, c.updated_at],
        )?;
        tick(&mut done);
    }

    for p in &snapshot.products {
        tx.execute(
            "INSERT INTO products
                (id, name, price, cost, photo_path, status, stock, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                p.id, p.name, p.price, p.cost, p.photo_path, p.status, p.stock, p.created_at, p.updated_at
            ],
        )?;
        tick(&mut done);
    }

    for s in &snapshot.services {
        tx.execute(
            "INSERT INTO services
                (id, name, description, price, duration_minutes, photo_path, status,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                s.id,
                s.name,
                s.description,
                s.price,
                s.duration_minutes,
                s.photo_path,
                s.status,
                s.created_at,
                s.updated_at,
            ],
        )?;
        tick(&mut done);
    }

    for pk in &snapshot.packages {
        tx.execute(
            "INSERT INTO packages (id, name, description, price, photo_path, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                pk.id, pk.name, pk.description, pk.price, pk.photo_path, pk.status, pk.created_at, pk.updated_at
            ],
        )?;
        tick(&mut done);
    }

    for e in &snapshot.employees {
        tx.execute(
            "INSERT INTO employees (id, name, color, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![e.id, e.name, e.color, e.active as i64, e.created_at, e.updated_at],
        )?;
        tick(&mut done);
    }

    for pi in &snapshot.package_items {
        tx.execute(
            "INSERT INTO package_items
                (id, package_id, item_type, product_id, service_id, quantity, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                pi.id, pi.package_id, pi.item_type, pi.product_id, pi.service_id, pi.quantity, pi.created_at
            ],
        )?;
        tick(&mut done);
    }

    // Mismo patrón que `write_snapshot_to_postgres`: `sale_id` se fuerza a
    // NULL acá y se restaura más abajo, una vez que `sales` ya existe.
    for a in &snapshot.appointments {
        tx.execute(
            "INSERT INTO appointments
                (id, employee_id, customer_id, service_id, service_name_snapshot,
                 starts_at, ends_at, status, notes, sale_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11)",
            rusqlite::params![
                a.id,
                a.employee_id,
                a.customer_id,
                a.service_id,
                a.service_name_snapshot,
                a.starts_at,
                a.ends_at,
                a.status,
                a.notes,
                a.created_at,
                a.updated_at,
            ],
        )?;
        tick(&mut done);
    }

    for s in &snapshot.sales {
        tx.execute(
            "INSERT INTO sales
                (id, customer_id, appointment_id, user_id, payment_method, total,
                 discount_amount, sold_at, created_at, voided_at, void_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                s.id,
                s.customer_id,
                s.appointment_id,
                s.user_id,
                s.payment_method,
                s.total,
                s.discount_amount,
                s.sold_at,
                s.created_at,
                s.voided_at,
                s.void_reason,
            ],
        )?;
        tick(&mut done);
    }

    for a in &snapshot.appointments {
        if let Some(sale_id) = &a.sale_id {
            tx.execute(
                "UPDATE appointments SET sale_id = ?1 WHERE id = ?2",
                rusqlite::params![sale_id, a.id],
            )?;
            tick(&mut done);
        }
    }

    for si in &snapshot.sale_items {
        tx.execute(
            "INSERT INTO sale_items
                (id, sale_id, item_type, product_id, service_id, package_id,
                 name_snapshot, unit_price, quantity, line_total, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                si.id,
                si.sale_id,
                si.item_type,
                si.product_id,
                si.service_id,
                si.package_id,
                si.name_snapshot,
                si.unit_price,
                si.quantity,
                si.line_total,
                si.created_at,
            ],
        )?;
        tick(&mut done);
    }

    for im in &snapshot.inventory_movements {
        tx.execute(
            "INSERT INTO inventory_movements
                (id, product_id, movement_type, quantity_delta, stock_after, sale_id, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                im.id,
                im.product_id,
                im.movement_type,
                im.quantity_delta,
                im.stock_after,
                im.sale_id,
                im.note,
                im.created_at,
            ],
        )?;
        tick(&mut done);
    }

    tx.commit()?;

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
        images_migrated: 0,
    })
}

/// Descarga del bucket cada imagen referenciada en el snapshot hacia
/// `<data_dir>/<photo_path>` -- espejo de [`migrate_images`] en la dirección
/// contraria. Igual que la subida, se corre ANTES de tocar SQLite: si alguna
/// descarga falla, no se escribe ninguna fila.
async fn download_images(
    data_dir: &Path,
    bucket_config: &crate::bucket_config::BucketConfig,
    bucket: &BucketClient,
    snapshot: &LocalSnapshot,
) -> AppResult<usize> {
    let paths = snapshot
        .products
        .iter()
        .filter_map(|p| p.photo_path.as_deref())
        .chain(snapshot.services.iter().filter_map(|s| s.photo_path.as_deref()))
        .chain(snapshot.packages.iter().filter_map(|pk| pk.photo_path.as_deref()));

    let mut count = 0;
    for photo_path in paths {
        let bytes = bucket.get_object(&bucket_config.object_key(photo_path)).await?;
        let dest = data_dir.join(photo_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Internal(format!("no se pudo crear el directorio de imágenes: {e}"))
            })?;
        }
        std::fs::write(&dest, bytes)
            .map_err(|e| AppError::Internal(format!("no se pudo guardar la imagen \"{photo_path}\": {e}")))?;
        count += 1;
    }
    Ok(count)
}

/// Punto de entrada usado por `storage_switch_mode` al volver a Local: lee
/// de PostgreSQL (async), descarga las imágenes del catálogo del bucket
/// configurado (si hay uno -- si no, se migra el resto igual, dejando
/// `images_migrated` en 0, mismo criterio que `migrate_local_to_postgres`) y
/// escribe los datos en una base SQLite nueva/vacía en `data_dir`, en un
/// hilo bloqueante (`spawn_blocking`, misma razón que `AppDb::with_sqlite`:
/// `rusqlite` es síncrono y no debe mezclarse con `.await`).
pub async fn migrate_postgres_to_local(
    on_progress: &mut (dyn FnMut(usize, usize) + Send),
    pg: &PgPool,
    data_dir: &Path,
    bucket: Option<(&crate::bucket_config::BucketConfig, &BucketClient)>,
    replace: bool,
) -> AppResult<MigrationSummary> {
    let snapshot = read_postgres_snapshot(pg).await?;

    let images_migrated = match bucket {
        Some((bucket_config, bucket_client)) => {
            download_images(data_dir, bucket_config, bucket_client, &snapshot).await?
        }
        None => 0,
    };

    let mut conn = crate::db::open(data_dir)
        .map_err(|e| AppError::Internal(format!("no se pudo abrir la base de datos local: {e}")))?;

    // `on_progress` es `&mut dyn FnMut + Send` (necesario para que el futuro
    // de `storage_switch_mode` sea `Send`, ver `commands::storage`), pero
    // `spawn_blocking` exige que la clausura que recibe sea `'static` --
    // `on_progress` es una referencia con vida propia del llamador, así que
    // no puede moverse dentro tal cual. Se corre directo en el hilo actual en
    // vez de `spawn_blocking`: a diferencia de `AppDb::with_sqlite` (pensado
    // para no bloquear el runtime de Tauri mientras atiende otros comandos),
    // acá ya estamos en medio de `storage_switch_mode`, un comando que de
    // todos modos bloquea su propia tarea hasta terminar.
    let mut summary = write_snapshot_to_local(on_progress, &mut conn, snapshot, replace)?;
    summary.images_migrated = images_migrated;
    Ok(summary)
}

/// Deja la base local vacía (mismo esquema, sin filas de negocio) después de
/// migrar sus datos a Postgres/Nube con éxito. Sin esto, la copia local queda
/// dando vueltas, cada vez más desactualizada respecto al destino remoto que
/// ahora es la fuente de verdad -- y además bloquearía una futura "migración
/// de vuelta" (`write_snapshot_to_local` exige un destino local vacío, igual
/// que `write_snapshot_to_postgres` exige lo mismo en la dirección
/// contraria): sin vaciar acá, volver de Nube a Local a mano requeriría
/// primero borrar el archivo local por fuera de la app.
///
/// Deliberadamente **no** toca `business_hours` (no es parte de ningún
/// snapshot — si se vaciara quedaría sin filas para siempre, ya que esa
/// tabla solo se siembra una vez, en la migración 0003, que no vuelve a
/// correr) ni `customers_search` (índice FTS, se vacía solo vía los
/// triggers de `customers`). Sí borra `sessions`, aunque esa tabla tampoco es
/// parte del snapshot (ver nota al principio del archivo): tokens de sesión
/// de una base que ya no es la activa no tienen ningún uso.
///
/// El orden importa: SQLite **ignora** cambios a `PRAGMA foreign_keys`
/// dentro de una transacción ya abierta (ver el comentario de
/// `db::open` sobre el mismo problema en las migraciones), así que esta
/// función no puede apoyarse en desactivarlo para borrar en cualquier orden
/// -- cuando se llama desde dentro de la transacción de
/// `write_snapshot_to_local` (camino "reemplazar"), `foreign_keys` sigue
/// `ON` todo el tiempo. En cambio, se borra en el único orden que de verdad
/// hace falta: antes que su tabla padre en las tres relaciones `ON DELETE
/// RESTRICT` del esquema (`sales.customer_id`, `appointments.employee_id`,
/// `appointments.customer_id`) -- todas las demás son `CASCADE`/`SET NULL`,
/// que no exigen ningún orden en particular.
pub fn clear_local_data(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "DELETE FROM inventory_movements;
         DELETE FROM sale_payments;
         DELETE FROM sale_items;
         DELETE FROM sales;
         DELETE FROM appointments;
         DELETE FROM package_items;
         DELETE FROM packages;
         DELETE FROM services;
         DELETE FROM products;
         DELETE FROM employee_availability;
         DELETE FROM employee_commission_rates;
         DELETE FROM employees;
         DELETE FROM cash_sessions;
         DELETE FROM customers;
         DELETE FROM sessions;
         DELETE FROM users;
         DELETE FROM business_profile;",
    )?;
    Ok(())
}
