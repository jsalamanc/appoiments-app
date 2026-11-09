use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::cognito::{self, CognitoClient, CognitoSessionInfo};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitoSession {
    pub sub: String,
    pub username: String,
    pub email: Option<String>,
    pub expires_at: String,
}

impl From<CognitoSessionInfo> for CognitoSession {
    fn from(info: CognitoSessionInfo) -> Self {
        let expires_at = time::OffsetDateTime::from_unix_timestamp(info.expires_at_unix)
            .ok()
            .and_then(|dt| dt.format(&time::format_description::well_known::Rfc3339).ok())
            .unwrap_or_default();
        Self {
            sub: info.sub,
            username: info.username,
            email: info.email,
            expires_at,
        }
    }
}

fn config_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|_| AppError::Internal("No se pudo resolver el directorio de configuración".into()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignUpInput {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignUpResult {
    pub needs_confirmation: bool,
}

#[tauri::command]
pub async fn cognito_signup(input: SignUpInput) -> AppResult<SignUpResult> {
    let client = cognito::CognitoClient::new();
    let needs_confirmation = client
        .sign_up(&input.username, &input.password, &input.email)
        .await?;
    Ok(SignUpResult { needs_confirmation })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmSignUpInput {
    pub username: String,
    pub code: String,
}

#[tauri::command]
pub async fn cognito_confirm_signup(input: ConfirmSignUpInput) -> AppResult<()> {
    let client = cognito::CognitoClient::new();
    client
        .confirm_sign_up(&input.username, &input.code)
        .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResendCodeInput {
    pub username: String,
}

#[tauri::command]
pub async fn cognito_resend_code(input: ResendCodeInput) -> AppResult<()> {
    let client = cognito::CognitoClient::new();
    client.resend_confirmation_code(&input.username).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordInput {
    pub username: String,
}

#[tauri::command]
pub async fn cognito_forgot_password(input: ForgotPasswordInput) -> AppResult<()> {
    let client = CognitoClient::new();
    client.forgot_password(&input.username).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmForgotPasswordInput {
    pub username: String,
    pub code: String,
    pub new_password: String,
}

#[tauri::command]
pub async fn cognito_confirm_forgot_password(input: ConfirmForgotPasswordInput) -> AppResult<()> {
    let client = CognitoClient::new();
    client
        .confirm_forgot_password(&input.username, &input.code, &input.new_password)
        .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitoLoginInput {
    pub username: String,
    pub password: String,
}

#[tauri::command]
pub async fn cognito_login(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CognitoLoginInput,
) -> AppResult<CognitoSession> {
    let client = CognitoClient::new();
    let tokens = client
        .initiate_auth_user_password(&input.username, &input.password)
        .await?;
    let session_info = CognitoSessionInfo::from_tokens(&tokens)?;

    cognito::set_tokens(&tokens)
        .map_err(|e| AppError::Internal(format!("No se pudo guardar la sesión: {e}")))?;
    *state.cognito_id_token.lock().unwrap() = Some(tokens.id_token.clone());

    let dir = config_dir(&app)?;
    let marker = cognito::CognitoInstallationMarker {
        established: true,
        username: session_info.username.clone(),
        sub: session_info.sub.clone(),
        last_login_at: cognito::now_iso(),
    };
    cognito::save_marker(&dir, &marker)
        .map_err(|e| AppError::Internal(format!("No se pudo guardar el estado de la instalación: {e}")))?;

    *state.cognito_session.lock().unwrap() = Some(session_info.clone());

    Ok(session_info.into())
}

#[tauri::command]
pub fn cognito_logout(app: AppHandle, state: State<AppState>) -> AppResult<()> {
    cognito::clear_tokens();
    if let Ok(dir) = config_dir(&app) {
        let _ = cognito::clear_marker(&dir);
    }
    *state.cognito_session.lock().unwrap() = None;
    *state.cognito_id_token.lock().unwrap() = None;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitoInstallationState {
    pub established: bool,
    pub session: Option<CognitoSession>,
}

#[tauri::command]
pub fn cognito_installation_state(
    app: AppHandle,
    state: State<AppState>,
) -> AppResult<CognitoInstallationState> {
    let established = config_dir(&app)
        .ok()
        .and_then(|dir| cognito::load_marker(&dir))
        .map(|marker| marker.established)
        .unwrap_or(false);
    let session = state
        .cognito_session
        .lock()
        .unwrap()
        .clone()
        .map(CognitoSession::from);
    Ok(CognitoInstallationState { established, session })
}
