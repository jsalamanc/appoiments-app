use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::app_db::AppDb;
use crate::audit::{self, NewEntry};
use crate::cognito::{self, CognitoClient, CognitoSessionInfo};
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
    let attempted_username = username.clone();
    let login = match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::login(conn, username, password, remember_me))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::login(pool, username, password, remember_me).await,
    };
    let (user, session_token) = match login {
        Ok(ok) => ok,
        Err(e) => {
            audit::record(
                &state.db,
                None,
                NewEntry::event(
                    "auth.login_failed",
                    None,
                    format!("Intento de inicio de sesión fallido para «{}»", attempted_username.trim()),
                ),
            )
            .await;
            return Err(e);
        }
    };

    *state.current_user.lock().unwrap() = Some(user.clone());
    audit::record(
        &state.db,
        Some(&user),
        NewEntry::event("auth.login", Some(&user.id), format!("Inició sesión «{}»", user.username)),
    )
    .await;

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
    let current_user = state.current_user.lock().unwrap().clone();
    let current_user_id = current_user.as_ref().map(|u| u.id.clone());

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
    if let Some(user) = &current_user {
        audit::record(
            &state.db,
            Some(user),
            NewEntry::event("auth.logout", Some(&user.id), format!("Cerró sesión «{}»", user.username)),
        )
        .await;
    }
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

    let target_user_id = input.user_id.clone();
    let tracker = audit::Tracker::start(&state.db, &current, "user.change_password", "users", Some(&target_user_id), &[]).await;
    let result = match &state.db {
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
    }?;
    tracker.detail(if current.id == target_user_id { "cambio de su propia contraseña" } else { "restablecida por el dueño" }).finish("Cambió contraseña de").await;
    Ok(result)
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

    let tracker = audit::Tracker::start(&state.db, &current, "user.create", "users", None, &[]).await;
    let result = match &state.db {
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
    }?;
    tracker.detail(format!("rol {}", result.role)).finish_created("Creó usuario", &result.id).await;
    Ok(result)
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

    let tracker = audit::Tracker::start(&state.db, &current, "user.set_active", "users", Some(&user_id), &[]).await;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::set_active(conn, user_id, active))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::set_active(pool, user_id, active).await,
    }?;
    tracker
        .finish(if active { "Activó usuario" } else { "Desactivó usuario" })
        .await;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverOwnerPasswordInput {
    pub cognito_email: String,
    pub cognito_password: String,
    pub local_username: String,
    pub new_password: String,
}

/// Recuperación del dueño local cuando olvidó su contraseña: en vez de
/// inventar un código de recuperación nuevo, se reusa la cuenta de Cognito
/// que ya configuró este negocio (capa totalmente aparte del login local,
/// ver `crate::cognito`) como prueba de identidad. Se exige que el `sub` de
/// quien se loguea coincida con el que quedó guardado en `cognito.json` al
/// hacer el setup inicial — si no coincide, cualquier cuenta de Cognito
/// válida (no solo la del dueño de este negocio) podría resetear la
/// contraseña local, lo cual sería un agujero de seguridad real.
#[tauri::command]
pub async fn auth_recover_owner_password(
    app: AppHandle,
    state: State<'_, AppState>,
    input: RecoverOwnerPasswordInput,
) -> AppResult<()> {
    let config_dir = app.path().app_config_dir().map_err(|_| {
        AppError::Internal("No se pudo resolver el directorio de configuración".into())
    })?;
    let marker = cognito::load_marker(&config_dir).ok_or_else(|| {
        AppError::validation("Esta instalación no tiene una cuenta de Cognito asociada")
    })?;

    let client = CognitoClient::new();
    let tokens = client
        .initiate_auth_user_password(&input.cognito_email, &input.cognito_password)
        .await?;
    let session = CognitoSessionInfo::from_tokens(&tokens)?;

    if session.sub != marker.sub {
        return Err(AppError::validation(
            "Esta cuenta de Cognito no es la que configuró este negocio",
        ));
    }

    let recovered_username = input.local_username.clone();
    let recovered_email = input.cognito_email.clone();
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| {
                    repo::reset_owner_password(conn, input.local_username, input.new_password)
                })
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::reset_owner_password(pool, input.local_username, input.new_password)
                .await
        }
    }?;
    audit::record(
        &state.db,
        None,
        NewEntry::event(
            "user.recover_owner_password",
            None,
            format!(
                "Recuperó la contraseña del dueño «{}» con la cuenta de Cognito {}",
                recovered_username.trim(),
                recovered_email.trim()
            ),
        ),
    )
    .await;
    Ok(())
}
