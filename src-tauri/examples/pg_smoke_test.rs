//! Verificación manual de extremo a extremo del backend PostgreSQL (issue
//! #9) contra un servidor real. No es parte de `cargo test` porque requiere
//! un Postgres corriendo y accesible — se ejecuta a mano con
//! `cargo run --example pg_smoke_test` cuando se cambie algo en la capa
//! `repositories::*::postgres` y se quiera confirmar que de verdad funciona
//! contra un servidor (no solo que compila).
//!
//! Requiere una base de datos VACÍA en 127.0.0.1:55432 (la migra sola) con:
//!   usuario "aura_test", contraseña "aura_test_pw", base "aura_studio_test".
//! Un Postgres de prueba así se puede levantar sin Docker ni root bajando los
//! .deb oficiales (`apt-get download postgresql-18 postgresql-client-18
//! postgresql-common postgresql-client-common libicu78 libpq5`),
//! extrayéndolos con `dpkg-deb -x` a un directorio propio, y corriendo
//! `initdb`/`pg_ctl` desde ahí como usuario normal.
//!
//! Si además existe `PG_SMOKE_TEST_CA_CERT` apuntando a un certificado CA, y
//! el servidor tiene SSL habilitado con un certificado firmado por esa CA,
//! también valida `sslmode=verify-full` con/sin certificado (soporte
//! agregado a pedido de un usuario real que notó que la UI no tenía forma de
//! subir un certificado CA). Para armar ese par CA+servidor con openssl:
//!   openssl genrsa -out ca.key 2048
//!   openssl req -new -x509 -days 3650 -key ca.key -out ca.crt \
//!     -subj "/CN=Test CA" -addext "basicConstraints=critical,CA:TRUE"
//!   openssl genrsa -out server.key 2048
//!   openssl req -new -key server.key -out server.csr -subj "/CN=127.0.0.1"
//!   openssl x509 -req -days 3650 -in server.csr -CA ca.crt -CAkey ca.key \
//!     -CAcreateserial -out server.crt -extfile <(printf \
//!     "basicConstraints=critical,CA:FALSE\nextendedKeyUsage=serverAuth\nsubjectAltName=IP:127.0.0.1")
//! (el cert del servidor NO puede ser el mismo archivo que la CA: rustls
//! rechaza con `CaUsedAsEndEntity` un certificado marcado CA:TRUE que
//! también se presenta como certificado de servidor).

use aura_studio_lib::app_db;
use aura_studio_lib::commands::appointments::AppointmentInput;
use aura_studio_lib::commands::business_profile::OnboardingInput;
use aura_studio_lib::commands::customers::CustomerInput;
use aura_studio_lib::commands::employees::EmployeeInput;
use aura_studio_lib::commands::packages::{PackageInput, PackageItemInput};
use aura_studio_lib::commands::products::ProductInput;
use aura_studio_lib::commands::sales::SaleItemInput;
use aura_studio_lib::commands::services::ServiceInput;
use aura_studio_lib::repositories::{
    appointments, auth, business_profile, customers, employees, packages, products, reports,
    sales, services,
};
use aura_studio_lib::storage_config::PostgresConnectionConfig;

fn main() {
    tauri::async_runtime::block_on(run_migration_checks()).expect("pruebas de migración fallaron");
    tauri::async_runtime::block_on(run()).expect("smoke test falló");

    match std::env::var("PG_SMOKE_TEST_CA_CERT") {
        Ok(path) => {
            tauri::async_runtime::block_on(run_ssl_check(std::path::Path::new(&path)))
                .expect("prueba de SSL/CA falló");
        }
        Err(_) => println!(
            "… PG_SMOKE_TEST_CA_CERT no está definida: se omite la prueba de verify-full/CA \
             (ver el comentario del encabezado de este archivo para armar un servidor con SSL)"
        ),
    }
}

/// Verifica el soporte de certificado CA (`verify-full`) agregado a pedido de
/// un usuario real que notó que la UI no tenía forma de subirlo: debe fallar
/// sin el certificado y conectar exitosamente con el correcto.
async fn run_ssl_check(ca_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = PostgresConnectionConfig {
        host: "127.0.0.1".into(),
        port: 55432,
        database: "aura_studio_test".into(),
        username: "aura_test".into(),
        sslmode: "verify-full".into(),
    };

    let without_cert = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(config.connect_options("aura_test_pw", None))
        .await;
    match without_cert {
        Err(_) => println!("✓ verify-full sin certificado CA falla como se espera"),
        Ok(_) => return Err("BUG: verify-full conectó sin certificado CA (no debería)".into()),
    }

    let with_cert = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(config.connect_options("aura_test_pw", Some(ca_path)))
        .await?;
    sqlx::query("SELECT 1").execute(&with_cert).await?;
    with_cert.close().await;
    println!("✓ verify-full con el certificado CA correcto conecta exitosamente");

    Ok(())
}

/// Verifica la migración automática de datos locales (issue #11): arma un
/// SQLite en memoria con una fila en cada una de las 12 tablas migradas
/// (incluida una cita y una venta que se referencian mutuamente, y un usuario
/// con una contraseña hasheada de verdad), migra hacia el Postgres de prueba
/// (debe estar vacío) y confirma los conteos, el enlace circular
/// appointments↔sales, y que el owner migrado puede loguearse con su
/// contraseña original (prueba de que `password_hash` nunca se re-hashea).
/// También confirma que un segundo intento de migración contra el mismo
/// destino (ya no vacío) se rechaza. Al final deja el Postgres vacío otra vez
/// (`TRUNCATE`) para que `run()` pueda arrancar desde cero, como ya
/// documenta la cabecera de este archivo.
async fn run_migration_checks() -> Result<(), Box<dyn std::error::Error>> {
    let config = PostgresConnectionConfig {
        host: "127.0.0.1".into(),
        port: 55432,
        database: "aura_studio_test".into(),
        username: "aura_test".into(),
        sslmode: "disable".into(),
    };
    let pool = app_db::open_postgres(&config, "aura_test_pw", None).await?;

    let mut conn = rusqlite::Connection::open_in_memory()?;
    aura_studio_lib::db::migrations().to_latest(&mut conn)?;

    let now = "2026-01-01T00:00:00Z";
    let hash = aura_studio_lib::auth::hash_password("migracion-test-123")?;
    conn.execute(
        "INSERT INTO business_profile
            (id, name, module_employees, module_appointments, module_services,
             module_packages, module_reports, created_at, updated_at)
         VALUES (1, 'Negocio Migrado', 1, 1, 1, 1, 1, ?1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO users (id, name, username, password_hash, role, active, created_at, updated_at)
         VALUES ('user-1', 'Owner Migrado', 'owner_migrado', ?1, 'owner', 1, ?2, ?2)",
        rusqlite::params![hash, now],
    )?;
    conn.execute(
        "INSERT INTO customers (id, name, phone, email, document, created_at, updated_at)
         VALUES ('cust-1', 'Cliente Migrado', '3000000001', 'migrado@test.com', '111111', ?1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO employees (id, name, color, active, created_at, updated_at)
         VALUES ('emp-1', 'Empleado Migrado', '#00ff00', 1, ?1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO products (id, name, price, status, stock, created_at, updated_at)
         VALUES ('prod-1', 'Producto Migrado', 9999.0, 'published', 5, ?1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO services (id, name, price, duration_minutes, status, created_at, updated_at)
         VALUES ('svc-1', 'Servicio Migrado', 5000.0, 45, 'published', ?1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO packages (id, name, price, status, created_at, updated_at)
         VALUES ('pkg-1', 'Paquete Migrado', 12000.0, 'published', ?1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO package_items (id, package_id, item_type, product_id, quantity, created_at)
         VALUES ('pkgitem-1', 'pkg-1', 'product', 'prod-1', 1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO appointments
            (id, employee_id, customer_id, service_id, service_name_snapshot,
             starts_at, ends_at, status, sale_id, created_at, updated_at)
         VALUES ('appt-1', 'emp-1', 'cust-1', 'svc-1', 'Servicio Migrado',
                 ?1, '2026-01-01T01:00:00Z', 'completed', 'sale-1', ?1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO sales (id, customer_id, appointment_id, payment_method, total, sold_at, created_at)
         VALUES ('sale-1', 'cust-1', 'appt-1', 'cash', 5000.0, ?1, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO sale_items
            (id, sale_id, item_type, service_id, name_snapshot, unit_price, quantity, line_total, created_at)
         VALUES ('saleitem-1', 'sale-1', 'service', 'svc-1', 'Servicio Migrado', 5000.0, 1, 5000.0, ?1)",
        rusqlite::params![now],
    )?;
    conn.execute(
        "INSERT INTO inventory_movements (id, product_id, movement_type, quantity_delta, stock_after, created_at)
         VALUES ('inv-1', 'prod-1', 'initial', 5, 5, ?1)",
        rusqlite::params![now],
    )?;

    let snapshot = aura_studio_lib::migration::read_local_snapshot(&mut conn)?;
    let summary = aura_studio_lib::migration::write_snapshot_to_postgres(&pool, snapshot).await?;
    if summary.customers != 1 || summary.appointments != 1 || summary.sales != 1 || summary.users != 1 {
        return Err(format!("BUG: conteos de migración inesperados: {summary:?}").into());
    }
    println!("✓ migración de datos locales: {summary:?}");

    let linked_sale_id: Option<String> =
        sqlx::query_scalar("SELECT sale_id FROM appointments WHERE id = $1")
            .bind("appt-1")
            .fetch_one(&pool)
            .await?;
    if linked_sale_id.as_deref() != Some("sale-1") {
        return Err("BUG: la cita migrada no quedó enlazada a la venta migrada".into());
    }
    println!("✓ el enlace circular appointments<->sales sobrevivió la migración");

    auth::postgres::login(&pool, "owner_migrado".into(), "migracion-test-123".into(), false).await?;
    println!(
        "✓ el owner migrado puede loguearse con su contraseña original \
         (password_hash no se re-hasheó)"
    );

    // Rechazo: el destino ya no está vacío (lo acabamos de poblar), un
    // segundo intento de migración debe fallar sin tocar nada.
    let mut conn2 = rusqlite::Connection::open_in_memory()?;
    aura_studio_lib::db::migrations().to_latest(&mut conn2)?;
    let empty_snapshot = aura_studio_lib::migration::read_local_snapshot(&mut conn2)?;
    match aura_studio_lib::migration::write_snapshot_to_postgres(&pool, empty_snapshot).await {
        Err(e) if e.to_string().contains("no está vacía") => {
            println!("✓ la migración rechaza un destino que ya tiene datos: {e}")
        }
        Err(e) => {
            return Err(format!("BUG: mensaje inesperado al rechazar destino no vacío: {e}").into())
        }
        Ok(_) => return Err("BUG: se permitió migrar hacia un destino con datos".into()),
    }

    // Deja la base vacía otra vez para que `run()` pueda arrancar desde cero.
    sqlx::query(
        "TRUNCATE TABLE sessions, inventory_movements, sale_items, sales, appointments, \
         package_items, packages, services, products, employees, customers, users, \
         business_profile RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await?;
    pool.close().await;

    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = PostgresConnectionConfig {
        host: "127.0.0.1".into(),
        port: 55432,
        database: "aura_studio_test".into(),
        username: "aura_test".into(),
        sslmode: "disable".into(),
    };
    let pool = app_db::open_postgres(&config, "aura_test_pw", None).await?;
    println!("✓ conectado y migrado contra Postgres real");

    let onboarding = business_profile::postgres::onboarding_complete_setup(
        &pool,
        OnboardingInput {
            business_name: "Salón de prueba".into(),
            owner_name: "Owner Test".into(),
            owner_username: "owner_test".into(),
            owner_password: "password123".into(),
        },
    )
    .await?;
    println!(
        "✓ onboarding: negocio={:?} owner={:?}",
        onboarding.profile.name, onboarding.owner.username
    );

    let (user, token) =
        auth::postgres::login(&pool, "owner_test".into(), "password123".into(), true).await?;
    println!("✓ login ok: user={} token_presente={}", user.username, token.is_some());

    let customer = customers::postgres::create(
        &pool,
        CustomerInput {
            name: "Cliente Test".into(),
            phone: "3000000000".into(),
            email: "cliente@test.com".into(),
            document: "123456".into(),
        },
    )
    .await?;
    println!("✓ customer creado: {} ({})", customer.name, customer.id);

    let employee = employees::postgres::create(
        &pool,
        EmployeeInput {
            name: "Empleado Test".into(),
            color: "#ff0000".into(),
            active: true,
        },
    )
    .await?;
    println!("✓ employee creado: {} ({})", employee.name, employee.id);

    let service = services::postgres::create(
        &pool,
        ServiceInput {
            name: "Corte de prueba".into(),
            description: None,
            price: 25000.0,
            duration_minutes: 30,
            photo_path: None,
            status: "published".into(),
        },
    )
    .await?;
    println!(
        "✓ service creado: {} ({} min, ${})",
        service.name, service.duration_minutes, service.price
    );

    let product = products::postgres::create(
        &pool,
        ProductInput {
            name: "Shampoo Test".into(),
            price: 15000.0,
            photo_path: None,
            status: "published".into(),
        },
    )
    .await?;
    println!("✓ product creado: {} ({})", product.name, product.id);

    let stock = products::postgres::set_stock(&pool, product.id.clone(), 10, Some("stock inicial".into())).await?;
    println!("✓ stock inicial del producto: {stock}");

    let package = packages::postgres::create(
        &pool,
        PackageInput {
            name: "Combo Test".into(),
            description: None,
            price: 35000.0,
            photo_path: None,
            status: "published".into(),
            items: vec![PackageItemInput {
                item_type: "product".into(),
                item_id: product.id.clone(),
                quantity: 1,
            }],
        },
    )
    .await?;
    println!(
        "✓ package creado: {} ({} ítems)",
        package.name,
        package.package_items.len()
    );

    let appointment = appointments::postgres::create(
        &pool,
        AppointmentInput {
            employee_id: employee.id.clone(),
            customer_id: customer.id.clone(),
            service_id: service.id.clone(),
            starts_at: "2026-06-01T15:00:00Z".into(),
            notes: Some("cita de prueba".into()),
        },
    )
    .await?;
    println!(
        "✓ appointment creado: {} -> {} (status={})",
        appointment.starts_at, appointment.ends_at, appointment.status
    );

    // Debe fallar: mismo empleado, mismo horario exacto (solape).
    let overlap_result = appointments::postgres::create(
        &pool,
        AppointmentInput {
            employee_id: employee.id.clone(),
            customer_id: customer.id.clone(),
            service_id: service.id.clone(),
            starts_at: "2026-06-01T15:00:00Z".into(),
            notes: None,
        },
    )
    .await;
    match overlap_result {
        Err(e) => println!("✓ detección de solape de citas funciona: {e}"),
        Ok(_) => return Err("BUG: se permitió una cita solapada".into()),
    }

    let sale_id = sales::postgres::create_with_inventory(
        &pool,
        customer.id.clone(),
        "cash".into(),
        vec![SaleItemInput {
            item_type: "product".into(),
            item_id: product.id.clone(),
            name: product.name.clone(),
            unit_price: product.price,
            quantity: 2,
        }],
    )
    .await?;
    println!("✓ venta creada: {sale_id}");

    let product_after = products::postgres::get(&pool, product.id.clone())
        .await?
        .ok_or("producto no encontrado tras la venta")?;
    if product_after.stock != 8 {
        return Err(format!(
            "BUG: stock esperado 8, encontrado {}",
            product_after.stock
        )
        .into());
    }
    println!("✓ descuento de inventario correcto: stock={}", product_after.stock);

    appointments::postgres::complete_with_sale(&pool, appointment.id.clone(), sale_id.clone())
        .await?;
    println!("✓ cita completada con venta (transacción cruzada appointments+sales)");

    let cash_summary = sales::postgres::cash_day_summary(&pool).await?;
    println!(
        "✓ resumen de caja del día: {} ventas, total=${}",
        cash_summary.sales_count, cash_summary.total_revenue
    );

    let metrics = sales::postgres::revenue_metrics(&pool).await?;
    println!(
        "✓ métricas de ingresos: total=${} mes=${} (sin bug de comparación de fechas)",
        metrics.total_revenue, metrics.month_revenue
    );

    let perf = reports::postgres::employee_performance(&pool, "month".into()).await?;
    println!("✓ reporte de desempeño de empleados: {} filas", perf.rows.len());

    let top_services = reports::postgres::top_services(&pool, "month".into()).await?;
    println!(
        "✓ reporte de top servicios: {} filas, ticket promedio=${}",
        top_services.rows.len(),
        top_services.average_ticket
    );

    let no_shows = reports::postgres::no_shows(&pool, "month".into()).await?;
    println!(
        "✓ reporte de no-shows: {} no-shows, {} cancelados",
        no_shows.no_show_total, no_shows.cancelled_total
    );

    let low_stock = products::postgres::list(&pool, Some("published".into()), None, None).await?;
    println!("✓ listado de productos publicados: {} items", low_stock.items.len());

    // --- Casos de error: el mapeo de errores de Postgres debe dar los
    // mismos mensajes amigables que ya da la versión SQLite, no errores
    // crudos de SQLSTATE. ---

    let dup_email = customers::postgres::create(
        &pool,
        CustomerInput {
            name: "Otro Cliente".into(),
            phone: "3111111111".into(),
            email: "cliente@test.com".into(), // mismo email que el customer ya creado
            document: "999999".into(),
        },
    )
    .await;
    match dup_email {
        Err(e) if e.to_string() == "Ya existe un cliente con ese correo" => {
            println!("✓ unicidad de email mapeada correctamente: {e}")
        }
        Err(e) => return Err(format!("BUG: mensaje de error inesperado para email duplicado: {e}").into()),
        Ok(_) => return Err("BUG: se permitió un email de cliente duplicado".into()),
    }

    let bad_login = auth::postgres::login(&pool, "owner_test".into(), "password_incorrecta".into(), false).await;
    match bad_login {
        Err(e) if e.to_string() == "Usuario o contraseña incorrectos" => {
            println!("✓ login con contraseña incorrecta mapeado correctamente: {e}")
        }
        Err(e) => return Err(format!("BUG: mensaje inesperado para login incorrecto: {e}").into()),
        Ok(_) => return Err("BUG: se permitió un login con contraseña incorrecta".into()),
    }

    let not_found = products::postgres::get(&pool, "00000000-0000-0000-0000-000000000000".into()).await?;
    if not_found.is_some() {
        return Err("BUG: se encontró un producto con un id inexistente".into());
    }
    println!("✓ producto inexistente devuelve None correctamente");

    println!("\n✅ TODO OK — smoke test completo contra Postgres real");
    Ok(())
}
