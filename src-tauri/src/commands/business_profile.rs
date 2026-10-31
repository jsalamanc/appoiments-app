use serde::Deserialize;
use tauri::State;

use crate::app_db::AppDb;
use crate::error::AppResult;
use crate::models::business_profile::{BusinessProfile, ModulesFlags};
use crate::models::user::UserPublic;
use crate::repositories::business_profile as repo;
use crate::state::AppState;

#[tauri::command]
pub async fn onboarding_is_complete(state: State<'_, AppState>) -> AppResult<bool> {
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(repo::onboarding_is_complete).await,
        AppDb::Postgres(pool) => repo::postgres::onboarding_is_complete(pool).await,
    }
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
pub async fn onboarding_complete_setup(
    state: State<'_, AppState>,
    input: OnboardingInput,
) -> AppResult<OnboardingResult> {
    let result = match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::onboarding_complete_setup(conn, input))
                .await?
        }
        AppDb::Postgres(pool) => repo::postgres::onboarding_complete_setup(pool, input).await?,
    };

    *state.current_user.lock().unwrap() = Some(result.owner.clone());

    Ok(result)
}

#[tauri::command]
pub async fn get_business_profile(state: State<'_, AppState>) -> AppResult<BusinessProfile> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(repo::get_profile).await,
        AppDb::Postgres(pool) => repo::postgres::get_profile(pool).await,
    }
}

#[tauri::command]
pub async fn update_business_profile(
    state: State<'_, AppState>,
    name: String,
) -> AppResult<BusinessProfile> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::update_profile(conn, name))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::update_profile(pool, name).await,
    }
}

#[tauri::command]
pub async fn get_enabled_modules(state: State<'_, AppState>) -> AppResult<ModulesFlags> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(repo::get_modules).await,
        AppDb::Postgres(pool) => repo::postgres::get_modules(pool).await,
    }
}

#[tauri::command]
pub async fn update_enabled_modules(
    state: State<'_, AppState>,
    modules: ModulesFlags,
) -> AppResult<ModulesFlags> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::update_modules(conn, modules))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::update_modules(pool, modules).await,
    }
}
