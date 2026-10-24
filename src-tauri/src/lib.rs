pub mod app_db;
pub mod auth;
pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod repositories;
pub mod state;
pub mod storage_config;
pub mod util;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("no se pudo resolver el directorio de datos de la app");
            let conn = db::open(&data_dir).expect("no se pudo abrir la base de datos local");
            app.manage(AppState::new(conn));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_login,
            commands::auth::auth_logout,
            commands::auth::auth_resume_session,
            commands::auth::auth_current_user,
            commands::auth::auth_change_password,
            commands::auth::create_staff_user,
            commands::auth::list_users,
            commands::auth::set_user_active,
            commands::business_profile::onboarding_is_complete,
            commands::business_profile::onboarding_complete_setup,
            commands::business_profile::get_business_profile,
            commands::business_profile::update_business_profile,
            commands::business_profile::get_enabled_modules,
            commands::business_profile::update_enabled_modules,
            commands::customers::list_customers,
            commands::customers::get_customer_detail,
            commands::customers::create_customer,
            commands::customers::update_customer,
            commands::customers::find_customer_by_document,
            commands::customers::find_customer_by_email,
            commands::customers::find_customer_by_id,
            commands::reports::get_employee_performance_report,
            commands::reports::get_top_services_report,
            commands::reports::get_no_shows_report,
            commands::employees::list_employees,
            commands::employees::create_employee,
            commands::employees::update_employee,
            commands::employees::delete_employee,
            commands::appointments::list_appointments_in_range,
            commands::appointments::create_appointment,
            commands::appointments::update_appointment,
            commands::appointments::set_appointment_status,
            commands::appointments::complete_appointment_with_sale,
            commands::appointments::list_published_services_for_appointments,
            commands::products::list_products,
            commands::products::get_product,
            commands::products::create_product,
            commands::products::update_product,
            commands::products::delete_product,
            commands::products::set_product_stock,
            commands::products::import_product_photo,
            commands::services::list_services,
            commands::services::get_service,
            commands::services::create_service,
            commands::services::update_service,
            commands::services::delete_service,
            commands::services::import_service_photo,
            commands::packages::list_packages,
            commands::packages::get_package,
            commands::packages::create_package,
            commands::packages::update_package,
            commands::packages::delete_package,
            commands::packages::import_package_photo,
            commands::inventory::list_low_stock_products,
            commands::inventory::list_inventory_movements,
            commands::sales::get_published_catalog,
            commands::sales::create_sale_with_inventory,
            commands::sales::list_sales,
            commands::sales::get_cash_day_summary,
            commands::sales::get_revenue_metrics,
            commands::sales::link_sale_to_appointment,
            commands::storage::storage_test_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación Tauri");
}
