use rusqlite::{params, Row};
use serde::Deserialize;
use tauri::State;

use crate::auth::hash_password;
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::business_profile::{BusinessProfile, ModulesFlags};
use crate::models::user::UserPublic;
use crate::state::AppState;

fn row_to_profile(row: &Row) -> rusqlite::Result<BusinessProfile> {
    Ok(BusinessProfile {
        name: row.get("name")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_modules(row: &Row) -> rusqlite::Result<ModulesFlags> {
    Ok(ModulesFlags {
        employees: row.get::<_, i64>("module_employees")? != 0,
        appointments: row.get::<_, i64>("module_appointments")? != 0,
        services: row.get::<_, i64>("module_services")? != 0,
        packages: row.get::<_, i64>("module_packages")? != 0,
        reports: row.get::<_, i64>("module_reports")? != 0,
    })
}

#[tauri::command]
pub fn onboarding_is_complete(state: State<AppState>) -> AppResult<bool> {
    let conn = state.db.lock().unwrap();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM business_profile", [], |r| r.get(0))?;
    Ok(count > 0)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingInput {
    pub business_name: String,
    pub owner_name: String,
    pub owner_username: String,
    pub owner_password: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingResult {
    pub profile: BusinessProfile,
    pub owner: UserPublic,
}

#[tauri::command]
pub fn onboarding_complete_setup(
    state: State<AppState>,
    input: OnboardingInput,
) -> AppResult<OnboardingResult> {
    if input.business_name.trim().is_empty() {
        return Err(AppError::validation("El nombre del negocio es obligatorio"));
    }
    if input.owner_name.trim().is_empty() || input.owner_username.trim().is_empty() {
        return Err(AppError::validation("Nombre y usuario del dueño son obligatorios"));
    }
    if input.owner_password.len() < 6 {
        return Err(AppError::validation(
            "La contraseña debe tener al menos 6 caracteres",
        ));
    }

    let mut conn = state.db.lock().unwrap();
    let existing: i64 = conn.query_row("SELECT COUNT(*) FROM business_profile", [], |r| r.get(0))?;
    if existing > 0 {
        return Err(AppError::conflict("El negocio ya fue configurado"));
    }

    let tx = conn.transaction()?;
    let now = now_iso();

    tx.execute(
        "INSERT INTO business_profile (id, name, created_at, updated_at) VALUES (1, ?1, ?2, ?2)",
        params![input.business_name.trim(), now],
    )?;

    let owner_id = new_id();
    let hash = hash_password(&input.owner_password)?;
    tx.execute(
        "INSERT INTO users (id, name, username, password_hash, role, active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'owner', 1, ?5, ?5)",
        params![
            owner_id,
            input.owner_name.trim(),
            input.owner_username.trim(),
            hash,
            now
        ],
    )?;

    let profile = tx.query_row(
        "SELECT * FROM business_profile WHERE id = 1",
        [],
        row_to_profile,
    )?;
    let owner = tx.query_row(
        "SELECT * FROM users WHERE id = ?1",
        params![owner_id],
        |row| {
            Ok(UserPublic {
                id: row.get("id")?,
                name: row.get("name")?,
                username: row.get("username")?,
                role: row.get("role")?,
                active: row.get::<_, i64>("active")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    )?;

    tx.commit()?;

    *state.current_user.lock().unwrap() = Some(owner.clone());

    Ok(OnboardingResult { profile, owner })
}

#[tauri::command]
pub fn get_business_profile(state: State<AppState>) -> AppResult<BusinessProfile> {
    let conn = state.db.lock().unwrap();
    let profile = conn.query_row("SELECT * FROM business_profile WHERE id = 1", [], row_to_profile)?;
    Ok(profile)
}

#[tauri::command]
pub fn update_business_profile(state: State<AppState>, name: String) -> AppResult<BusinessProfile> {
    if name.trim().is_empty() {
        return Err(AppError::validation("El nombre del negocio es obligatorio"));
    }
    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE business_profile SET name = ?1, updated_at = ?2 WHERE id = 1",
        params![name.trim(), now_iso()],
    )?;
    let profile = conn.query_row("SELECT * FROM business_profile WHERE id = 1", [], row_to_profile)?;
    Ok(profile)
}

#[tauri::command]
pub fn get_enabled_modules(state: State<AppState>) -> AppResult<ModulesFlags> {
    let conn = state.db.lock().unwrap();
    let modules = conn.query_row(
        "SELECT * FROM business_profile WHERE id = 1",
        [],
        row_to_modules,
    )?;
    Ok(modules)
}

#[tauri::command]
pub fn update_enabled_modules(
    state: State<AppState>,
    modules: ModulesFlags,
) -> AppResult<ModulesFlags> {
    let normalized = modules.normalize();
    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE business_profile SET
            module_employees = ?1,
            module_appointments = ?2,
            module_services = ?3,
            module_packages = ?4,
            module_reports = ?5,
            updated_at = ?6
         WHERE id = 1",
        params![
            normalized.employees as i64,
            normalized.appointments as i64,
            normalized.services as i64,
            normalized.packages as i64,
            normalized.reports as i64,
            now_iso(),
        ],
    )?;
    Ok(normalized)
}
