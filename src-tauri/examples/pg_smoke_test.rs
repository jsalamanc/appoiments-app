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
    tauri::async_runtime::block_on(run()).expect("smoke test falló");
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = PostgresConnectionConfig {
        host: "127.0.0.1".into(),
        port: 55432,
        database: "aura_studio_test".into(),
        username: "aura_test".into(),
        sslmode: "disable".into(),
    };
    let pool = app_db::open_postgres(&config, "aura_test_pw").await?;
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
