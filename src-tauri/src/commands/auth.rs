use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::auth::{generate_session_token, hash_password, verify_password};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::user::UserPublic;
use crate::state::AppState;

const SESSION_TTL_DAYS: i64 = 30;

fn row_to_user(row: &Row) -> rusqlite::Result<UserPublic> {
    Ok(UserPublic {
        id: row.get("id")?,
        name: row.get("name")?,
        username: row.get("username")?,
        role: row.get("role")?,
        active: row.get::<_, i64>("active")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub user: UserPublic,
    pub session_token: Option<String>,
}

#[tauri::command]
pub fn auth_login(
    state: State<AppState>,
    username: String,
    password: String,
    remember_me: bool,
) -> AppResult<LoginResult> {
    let conn = state.db.lock().unwrap();

    let username_norm = username.trim().to_lowercase();
    let mut stmt = conn.prepare(
        "SELECT * FROM users WHERE lower(username) = ?1 AND active = 1",
    )?;
    let user_row = stmt
        .query_row(params![username_norm], |row| {
            let user = row_to_user(row)?;
            let hash: String = row.get("password_hash")?;
            Ok((user, hash))
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::validation("Usuario o contraseña incorrectos")
            }
            other => AppError::from(other),
        })?;

    let (user, password_hash) = user_row;

    if !verify_password(&password, &password_hash) {
        return Err(AppError::validation("Usuario o contraseña incorrectos"));
    }

    let session_token = if remember_me {
        let token = generate_session_token();
        let now = now_iso();
        let expires_at = time::OffsetDateTime::now_utc()
            .saturating_add(time::Duration::days(SESSION_TTL_DAYS))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or(now);
        conn.execute(
            "INSERT INTO sessions (token, user_id, expires_at) VALUES (?1, ?2, ?3)",
            params![token, user.id, expires_at],
        )?;
        Some(token)
    } else {
        None
    };

    drop(conn);
    *state.current_user.lock().unwrap() = Some(user.clone());

    Ok(LoginResult {
        user,
        session_token,
    })
}

#[tauri::command]
pub fn auth_logout(state: State<AppState>, session_token: Option<String>) -> AppResult<()> {
    if let Some(token) = session_token {
        let conn = state.db.lock().unwrap();
        conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
    }
    *state.current_user.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub fn auth_resume_session(
    state: State<AppState>,
    session_token: String,
) -> AppResult<Option<UserPublic>> {
    let conn = state.db.lock().unwrap();
    let now = now_iso();

    let mut stmt = conn.prepare(
        "SELECT u.* FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.token = ?1 AND s.expires_at > ?2 AND u.active = 1",
    )?;
    let user = stmt
        .query_row(params![session_token, now], row_to_user)
        .ok();

    drop(conn);
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
pub fn auth_change_password(
    state: State<AppState>,
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

    let conn = state.db.lock().unwrap();

    if !is_owner || current.id == input.user_id {
        let existing_hash: String = conn.query_row(
            "SELECT password_hash FROM users WHERE id = ?1",
            params![input.user_id],
            |row| row.get(0),
        )?;
        let old = input.old_password.unwrap_or_default();
        if !verify_password(&old, &existing_hash) {
            return Err(AppError::validation("La contraseña actual no es correcta"));
        }
    }

    if input.new_password.len() < 6 {
        return Err(AppError::validation(
            "La nueva contraseña debe tener al menos 6 caracteres",
        ));
    }

    let new_hash = hash_password(&input.new_password)?;
    conn.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
        params![new_hash, now_iso(), input.user_id],
    )?;

    Ok(())
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
pub fn create_staff_user(
    state: State<AppState>,
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
    let role = if input.role == "owner" { "owner" } else { "staff" };

    let conn = state.db.lock().unwrap();
    let id = new_id();
    let now = now_iso();
    let hash = hash_password(&input.password)?;

    conn.execute(
        "INSERT INTO users (id, name, username, password_hash, role, active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
        params![id, input.name.trim(), input.username.trim(), hash, role, now],
    )?;

    let user = conn.query_row("SELECT * FROM users WHERE id = ?1", params![id], row_to_user)?;
    Ok(user)
}

#[tauri::command]
pub fn list_users(state: State<AppState>) -> AppResult<Vec<UserPublic>> {
    let current = state
        .current_user
        .lock()
        .unwrap()
        .clone()
        .ok_or(AppError::Unauthorized)?;
    if current.role != "owner" {
        return Err(AppError::Unauthorized);
    }

    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT * FROM users ORDER BY created_at ASC")?;
    let rows = stmt.query_map([], row_to_user)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

#[tauri::command]
pub fn set_user_active(state: State<AppState>, user_id: String, active: bool) -> AppResult<()> {
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
        return Err(AppError::validation("No puedes desactivar tu propio usuario"));
    }

    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE users SET active = ?1, updated_at = ?2 WHERE id = ?3",
        params![active as i64, now_iso(), user_id],
    )?;
    Ok(())
}
