use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_db::AppDb;
use crate::error::{AppError, AppResult};
use crate::models::user::UserPublic;
use crate::repositories::auth as repo;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub user: UserPublic,
    pub session_token: Option<String>,
}

#[tauri::command]
pub async fn auth_login(
    state: State<'_, AppState>,
    username: String,
    password: String,
    remember_me: bool,
) -> AppResult<LoginResult> {
    let (user, session_token) = match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::login(conn, username, password, remember_me))
                .await?
        }
        AppDb::Postgres(pool) => repo::postgres::login(pool, username, password, remember_me).await?,
    };

    *state.current_user.lock().unwrap() = Some(user.clone());

    Ok(LoginResult {
        user,
        session_token,
    })
}

/// Issue #2 (auditoría de permisos): `delete_session` ahora exige que el
/// `session_token` pertenezca al usuario actualmente reconocido por este
/// proceso (`current_user`) — antes borraba cualquier token que se le
/// pasara, sin importar de quién fuera, así que un llamado con el token de
/// otra persona podía cerrarle la sesión sin su consentimiento.
#[tauri::command]
pub async fn auth_logout(
    state: State<'_, AppState>,
    session_token: Option<String>,
) -> AppResult<()> {
    let current_user_id = state.current_user.lock().unwrap().as_ref().map(|u| u.id.clone());

    if let (Some(token), Some(user_id)) = (session_token, current_user_id) {
        match &state.db {
            AppDb::Sqlite(_) => {
                state
                    .db
                    .with_sqlite(move |conn| repo::delete_session(conn, token, user_id))
                    .await?
            }
            AppDb::Postgres(pool) => repo::postgres::delete_session(pool, token, user_id).await?,
        }
    }
    *state.current_user.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub async fn auth_resume_session(
    state: State<'_, AppState>,
    session_token: String,
) -> AppResult<Option<UserPublic>> {
    let user = match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::resume_session(conn, session_token))
                .await?
        }
        AppDb::Postgres(pool) => repo::postgres::resume_session(pool, session_token).await?,
    };

    if let Some(ref u) = user {
        *state.current_user.lock().unwrap() = Some(u.clone());
    }
    Ok(user)
}

#[tauri::command]
pub fn auth_current_user(state: State<AppState>) -> AppResult<Option<UserPublic>> {
    Ok(state.current_user.lock().unwrap().clone())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordInput {
    pub user_id: String,
    pub old_password: Option<String>,
    pub new_password: String,
}

#[tauri::command]
pub async fn auth_change_password(
    state: State<'_, AppState>,
    input: ChangePasswordInput,
) -> AppResult<()> {
    let current = state
        .current_user
        .lock()
        .unwrap()
        .clone()
        .ok_or(AppError::Unauthorized)?;

    let is_owner = current.role == "owner";
    if !is_owner && current.id != input.user_id {
        return Err(AppError::Unauthorized);
    }
    let verify_old_password = !is_owner || current.id == input.user_id;

    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| {
                    repo::change_password(
                        conn,
                        input.user_id,
                        input.old_password,
                        input.new_password,
                        verify_old_password,
                    )
                })
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::change_password(
                pool,
                input.user_id,
                input.old_password,
                input.new_password,
                verify_old_password,
            )
            .await
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStaffInput {
    pub name: String,
    pub username: String,
    pub password: String,
    pub role: String,
}

#[tauri::command]
pub async fn create_staff_user(
    state: State<'_, AppState>,
    input: CreateStaffInput,
) -> AppResult<UserPublic> {
    let current = state
        .current_user
        .lock()
        .unwrap()
        .clone()
        .ok_or(AppError::Unauthorized)?;
    if current.role != "owner" {
        return Err(AppError::Unauthorized);
    }

    if input.name.trim().is_empty() || input.username.trim().is_empty() {
        return Err(AppError::validation("Nombre y usuario son obligatorios"));
    }
    if input.password.len() < 6 {
        return Err(AppError::validation(
            "La contraseña debe tener al menos 6 caracteres",
        ));
    }
    let role = if input.role == "owner" {
        "owner"
    } else {
        "staff"
    };

    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| {
                    repo::create_staff(conn, input.name, input.username, input.password, role)
                })
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::create_staff(pool, input.name, input.username, input.password, role)
                .await
        }
    }
}

#[tauri::command]
pub async fn list_users(state: State<'_, AppState>) -> AppResult<Vec<UserPublic>> {
    let current = state
        .current_user
        .lock()
        .unwrap()
        .clone()
        .ok_or(AppError::Unauthorized)?;
    if current.role != "owner" {
        return Err(AppError::Unauthorized);
    }

    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(repo::list).await,
        AppDb::Postgres(pool) => repo::postgres::list(pool).await,
    }
}

#[tauri::command]
pub async fn set_user_active(
    state: State<'_, AppState>,
    user_id: String,
    active: bool,
) -> AppResult<()> {
    let current = state
        .current_user
        .lock()
        .unwrap()
        .clone()
        .ok_or(AppError::Unauthorized)?;
    if current.role != "owner" {
        return Err(AppError::Unauthorized);
    }
    if current.id == user_id && !active {
        return Err(AppError::validation(
            "No puedes desactivar tu propio usuario",
        ));
    }

    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::set_active(conn, user_id, active))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::set_active(pool, user_id, active).await,
    }
}
