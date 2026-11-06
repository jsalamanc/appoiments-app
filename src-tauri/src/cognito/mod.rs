//! Cliente mínimo contra la API de Cognito Identity Provider (`InitiateAuth`,
//! `SignUp`, `ConfirmSignUp`, `ResendConfirmationCode`,
//! `RefreshToken`/`REFRESH_TOKEN_AUTH`) y persistencia de la sesión.
//!
//! Deliberadamente no se usa el SDK de AWS: el App Client es público (sin
//! secret), así que todo lo que hace falta es mandar JSON con el header
//! `X-Amz-Target` — mismo espíritu liviano que `util::bucket` para S3.
//!
//! Capa totalmente independiente del login local (`auth`/`commands::auth`):
//! esta sesión identifica a quien administra la suscripción/negocio en la
//! nube, no a quien opera la app (eso lo sigue resolviendo el sistema
//! owner/staff existente, sin cambios).

use std::fs;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::error::{AppError, AppResult};

// Outputs reales de `terraform output` en
// `appoiments-cloud-backend/infra/environments/{dev,prod}` (cuenta AWS
// 253320031976, dos User Pools de Cognito separados). No son secretos — son
// identificadores públicos, visibles igual en el `issuer`/`aud` de cualquier
// JWT emitido — pero igual van por variable de entorno (resuelta en tiempo
// de COMPILACIÓN vía `env!`, no en runtime) porque el valor correcto
// depende de donde se compile: local (`.env.local`, ver build.rs y
// `.env.local.example`) trae los de dev, el workflow de release trae los de
// prod puestos directo en el step de CI.
const USER_POOL_ID: &str = env!("COGNITO_USER_POOL_ID");
const APP_CLIENT_ID: &str = env!("COGNITO_APP_CLIENT_ID");

const KEYRING_SERVICE: &str = "com.aurastudio.cognito";
const CONFIG_FILE_NAME: &str = "cognito.json";

/// Margen antes del vencimiento real del `id_token` en el que ya se prefiere
/// refrescar en vez de esperar a que venza — evita que una llamada que
/// arranca justo antes del vencimiento llegue a Cognito con un token ya
/// inválido.
const REFRESH_MARGIN_SECONDS: i64 = 5 * 60;

fn region_from_pool_id(pool_id: &str) -> &str {
    pool_id.split('_').next().unwrap_or("us-east-1")
}

fn endpoint() -> String {
    format!(
        "https://cognito-idp.{}.amazonaws.com/",
        region_from_pool_id(USER_POOL_ID)
    )
}

pub fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

// --- Tokens devueltos por Cognito -------------------------------------------------

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub id_token: String,
    pub access_token: String,
    /// Ausente en la respuesta de un refresh (Cognito no rota el refresh
    /// token en ese flujo): quien llame conserva el que ya tenía guardado.
    pub refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthenticationResultResponse {
    #[serde(rename = "IdToken")]
    id_token: String,
    #[serde(rename = "AccessToken")]
    access_token: String,
    #[serde(rename = "RefreshToken")]
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InitiateAuthResponse {
    #[serde(rename = "AuthenticationResult")]
    authentication_result: Option<AuthenticationResultResponse>,
}

#[derive(Debug, Deserialize)]
struct SignUpResponse {
    #[serde(rename = "UserConfirmed")]
    user_confirmed: bool,
}

// --- Claims del id_token (solo lectura local, sin verificar firma: la firma
// solo le importa al backend, que sí la valida con el authorizer JWT) -------

#[derive(Debug, Clone, Deserialize)]
struct JwtClaims {
    sub: String,
    exp: i64,
    #[serde(default)]
    email: Option<String>,
    #[serde(rename = "cognito:username", default)]
    cognito_username: Option<String>,
}

fn decode_jwt_claims(token: &str) -> AppResult<JwtClaims> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or_else(|| AppError::Internal("Token de Cognito con formato inválido".into()))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| AppError::Internal(format!("No se pudo decodificar el token: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| AppError::Internal(format!("Token de Cognito con contenido inválido: {e}")))
}

/// Sesión de Cognito ya resuelta (claims decodificados), lo que se guarda en
/// `AppState` en memoria. No contiene los tokens crudos.
#[derive(Debug, Clone)]
pub struct CognitoSessionInfo {
    pub sub: String,
    pub username: String,
    pub email: Option<String>,
    pub expires_at_unix: i64,
}

impl CognitoSessionInfo {
    pub fn from_tokens(tokens: &AuthTokens) -> AppResult<Self> {
        decode_jwt_claims(&tokens.id_token).map(|claims| Self::from_claims(&claims))
    }

    fn from_claims(claims: &JwtClaims) -> Self {
        Self {
            sub: claims.sub.clone(),
            username: claims
                .cognito_username
                .clone()
                .unwrap_or_else(|| claims.sub.clone()),
            email: claims.email.clone(),
            expires_at_unix: claims.exp,
        }
    }
}

// --- Marcador no sensible de la instalación ("¿ya pasó por el gate de
// Cognito?"), separado de `storage.json` ------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitoInstallationMarker {
    pub established: bool,
    pub username: String,
    pub sub: String,
    pub last_login_at: String,
}

fn marker_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CONFIG_FILE_NAME)
}

pub fn load_marker(config_dir: &Path) -> Option<CognitoInstallationMarker> {
    fs::read_to_string(marker_path(config_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

pub fn save_marker(config_dir: &Path, marker: &CognitoInstallationMarker) -> std::io::Result<()> {
    fs::create_dir_all(config_dir)?;
    let raw =
        serde_json::to_string_pretty(marker).expect("CognitoInstallationMarker siempre serializa");
    fs::write(marker_path(config_dir), raw)
}

pub fn clear_marker(config_dir: &Path) -> std::io::Result<()> {
    match fs::remove_file(marker_path(config_dir)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

// --- Keychain: tokens guardados por separado, nunca en storage.json/localStorage ---

fn keyring_entry(account: &str) -> keyring::Result<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, account)
}

/// Guarda `id_token`/`access_token` siempre, y `refresh_token` solo si vino
/// en la respuesta (login inicial; un refresh normalmente no trae uno nuevo,
/// así que no se pisa el que ya había).
pub fn set_tokens(tokens: &AuthTokens) -> keyring::Result<()> {
    keyring_entry("id_token")?.set_password(&tokens.id_token)?;
    keyring_entry("access_token")?.set_password(&tokens.access_token)?;
    if let Some(refresh_token) = &tokens.refresh_token {
        keyring_entry("refresh_token")?.set_password(refresh_token)?;
    }
    Ok(())
}

struct StoredTokens {
    id_token: String,
    refresh_token: String,
}

fn get_tokens() -> keyring::Result<StoredTokens> {
    Ok(StoredTokens {
        id_token: keyring_entry("id_token")?.get_password()?,
        refresh_token: keyring_entry("refresh_token")?.get_password()?,
    })
}

pub fn clear_tokens() {
    for account in ["id_token", "access_token", "refresh_token"] {
        if let Ok(entry) = keyring_entry(account) {
            let _ = entry.delete_credential();
        }
    }
}

// --- Cliente HTTP ------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CognitoErrorBody {
    #[serde(rename = "__type")]
    error_type: Option<String>,
    message: Option<String>,
}

/// Error crudo de Cognito, con el nombre de la excepción todavía disponible
/// (sin mapear a español) — algunos flujos necesitan decidir su propia
/// respuesta según la excepción exacta (ej. `forgot_password` no debe
/// revelar si el usuario existe o no, así que trata
/// `UserNotFoundException` distinto a como la trataría `map_cognito_error`).
struct CognitoApiError {
    exception: String,
    message: String,
}

fn map_cognito_error(err: &CognitoApiError) -> AppError {
    match err.exception.as_str() {
        "UsernameExistsException" => {
            AppError::conflict("Ya existe una cuenta con ese usuario o correo")
        }
        "InvalidPasswordException" => AppError::validation(
            "La contraseña no cumple los requisitos mínimos (mayúsculas, minúsculas y números)",
        ),
        "InvalidParameterException" => AppError::validation(if err.message.is_empty() {
            "Datos inválidos".to_string()
        } else {
            err.message.clone()
        }),
        "CodeMismatchException" => AppError::validation("Código de confirmación incorrecto"),
        "ExpiredCodeException" => AppError::validation("El código venció, solicita uno nuevo"),
        "UserNotConfirmedException" => {
            AppError::validation("Confirma tu cuenta antes de iniciar sesión (revisa tu correo)")
        }
        "NotAuthorizedException" | "UserNotFoundException" => {
            AppError::validation("Usuario o contraseña incorrectos")
        }
        "LimitExceededException" | "TooManyRequestsException" | "TooManyFailedAttemptsException" => {
            AppError::validation("Demasiados intentos, esperá un momento antes de volver a intentar")
        }
        _ => AppError::Internal(format!("Cognito: {}", err.message)),
    }
}

pub struct CognitoClient {
    http: reqwest::Client,
}

impl CognitoClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Manda la petición y, si Cognito respondió un error, lo devuelve crudo
    /// (exception + message) sin mapear todavía — ver `CognitoApiError`.
    async fn send<B: Serialize>(
        &self,
        action: &str,
        body: &B,
    ) -> Result<reqwest::Response, CognitoApiError> {
        let response = self
            .http
            .post(endpoint())
            .header("Content-Type", "application/x-amz-json-1.1")
            .header(
                "X-Amz-Target",
                format!("AWSCognitoIdentityProviderService.{action}"),
            )
            .json(body)
            .send()
            .await
            .map_err(|e| CognitoApiError {
                exception: String::new(),
                message: format!("No se pudo contactar a Cognito: {e}"),
            })?;

        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        let parsed: Option<CognitoErrorBody> = serde_json::from_str(&body_text).ok();
        let error_type = parsed
            .as_ref()
            .and_then(|b| b.error_type.clone())
            .unwrap_or_default();
        // El `__type` a veces viene con el namespace del servicio como
        // prefijo (ej. "com.amazonaws...#UsernameExistsException") — nos
        // quedamos con lo que sigue al último '#'.
        let exception = error_type
            .rsplit('#')
            .next()
            .unwrap_or(&error_type)
            .to_string();
        let message = parsed
            .and_then(|b| b.message)
            .unwrap_or_else(|| format!("Cognito respondió {status}"));

        Err(CognitoApiError { exception, message })
    }

    async fn call<B: Serialize>(&self, action: &str, body: &B) -> AppResult<reqwest::Response> {
        self.send(action, body)
            .await
            .map_err(|e| map_cognito_error(&e))
    }

    /// Devuelve `true` si Cognito todavía exige confirmar la cuenta con un
    /// código (comportamiento por defecto del User Pool, con verificación de
    /// email habilitada).
    pub async fn sign_up(&self, username: &str, password: &str, email: &str) -> AppResult<bool> {
        let response = self
            .call(
                "SignUp",
                &serde_json::json!({
                    "ClientId": APP_CLIENT_ID,
                    "Username": username,
                    "Password": password,
                    "UserAttributes": [{ "Name": "email", "Value": email }],
                }),
            )
            .await?;
        let parsed: SignUpResponse = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Respuesta de SignUp inválida: {e}")))?;
        Ok(!parsed.user_confirmed)
    }

    pub async fn confirm_sign_up(&self, username: &str, code: &str) -> AppResult<()> {
        self.call(
            "ConfirmSignUp",
            &serde_json::json!({
                "ClientId": APP_CLIENT_ID,
                "Username": username,
                "ConfirmationCode": code,
            }),
        )
        .await?;
        Ok(())
    }

    pub async fn resend_confirmation_code(&self, username: &str) -> AppResult<()> {
        self.call(
            "ResendConfirmationCode",
            &serde_json::json!({
                "ClientId": APP_CLIENT_ID,
                "Username": username,
            }),
        )
        .await?;
        Ok(())
    }

    /// Pide a Cognito que mande el código de recuperación de contraseña.
    /// Deliberadamente trata `UserNotFoundException` como éxito silencioso:
    /// si se propagara como error, el frontend podría distinguir "ese
    /// usuario no existe" de "código enviado", lo que permite enumerar
    /// cuentas probando usuarios al voleo. Con esto, la respuesta es
    /// indistinguible exista o no la cuenta.
    pub async fn forgot_password(&self, username: &str) -> AppResult<()> {
        match self
            .send(
                "ForgotPassword",
                &serde_json::json!({
                    "ClientId": APP_CLIENT_ID,
                    "Username": username,
                }),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if e.exception == "UserNotFoundException" => Ok(()),
            Err(e) => Err(map_cognito_error(&e)),
        }
    }

    /// Confirma el código de recuperación y fija la nueva contraseña. Mismo
    /// cuidado que `forgot_password`: un `UserNotFoundException` acá se
    /// reporta como código incorrecto, no como "no existe esa cuenta".
    pub async fn confirm_forgot_password(
        &self,
        username: &str,
        code: &str,
        new_password: &str,
    ) -> AppResult<()> {
        match self
            .send(
                "ConfirmForgotPassword",
                &serde_json::json!({
                    "ClientId": APP_CLIENT_ID,
                    "Username": username,
                    "ConfirmationCode": code,
                    "Password": new_password,
                }),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if e.exception == "UserNotFoundException" => {
                Err(AppError::validation("Código de confirmación incorrecto"))
            }
            Err(e) => Err(map_cognito_error(&e)),
        }
    }

    async fn initiate_auth(&self, body: serde_json::Value) -> AppResult<AuthTokens> {
        let response = self.call("InitiateAuth", &body).await?;
        let parsed: InitiateAuthResponse = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Respuesta de InitiateAuth inválida: {e}")))?;
        let result = parsed.authentication_result.ok_or_else(|| {
            AppError::validation(
                "Cognito pidió un paso adicional no soportado por esta app (ej. cambio de contraseña obligatorio)",
            )
        })?;
        Ok(AuthTokens {
            id_token: result.id_token,
            access_token: result.access_token,
            refresh_token: result.refresh_token,
        })
    }

    pub async fn initiate_auth_user_password(
        &self,
        username: &str,
        password: &str,
    ) -> AppResult<AuthTokens> {
        self.initiate_auth(serde_json::json!({
            "AuthFlow": "USER_PASSWORD_AUTH",
            "ClientId": APP_CLIENT_ID,
            "AuthParameters": {
                "USERNAME": username,
                "PASSWORD": password,
            },
        }))
        .await
    }

    pub async fn refresh(&self, refresh_token: &str) -> AppResult<AuthTokens> {
        self.initiate_auth(serde_json::json!({
            "AuthFlow": "REFRESH_TOKEN_AUTH",
            "ClientId": APP_CLIENT_ID,
            "AuthParameters": {
                "REFRESH_TOKEN": refresh_token,
            },
        }))
        .await
    }
}

impl Default for CognitoClient {
    fn default() -> Self {
        Self::new()
    }
}

// --- Resolución de sesión al arrancar la app ---------------------------------------

/// Núcleo compartido entre `resolve_session_on_startup` (arranque) y
/// `ensure_fresh_id_token` (bajo demanda, para llamadas a `vendor::`): dado
/// el marcador + tokens guardados, decide si el `id_token` sigue fresco o si
/// hace falta refrescarlo en silencio, y devuelve la sesión resuelta junto
/// con un `id_token` válido. Una sola implementación del refresh, no dos
/// copias.
async fn resolve_or_refresh(config_dir: &Path) -> Option<(CognitoSessionInfo, String)> {
    let marker = load_marker(config_dir)?;
    if !marker.established {
        return None;
    }

    let stored = get_tokens().ok()?;
    let claims = decode_jwt_claims(&stored.id_token).ok()?;

    let now = OffsetDateTime::now_utc().unix_timestamp();
    if claims.exp - now > REFRESH_MARGIN_SECONDS {
        // Todavía fresco: nada de red, esto es lo que hace que reabrir la
        // app (o pedir un token para `vendor::`) sea instantáneo la mayoría
        // de las veces.
        return Some((CognitoSessionInfo::from_claims(&claims), stored.id_token));
    }

    let client = CognitoClient::new();
    match client.refresh(&stored.refresh_token).await {
        Ok(new_tokens) => {
            let refreshed_claims = decode_jwt_claims(&new_tokens.id_token).ok()?;
            let fresh_id_token = new_tokens.id_token.clone();
            let _ = set_tokens(&AuthTokens {
                id_token: new_tokens.id_token,
                access_token: new_tokens.access_token,
                // `REFRESH_TOKEN_AUTH` no rota el refresh token: se conserva
                // el que ya estaba guardado (`set_tokens` no lo toca si viene
                // `None`).
                refresh_token: None,
            });
            let mut updated_marker = marker;
            updated_marker.last_login_at = now_iso();
            let _ = save_marker(config_dir, &updated_marker);
            Some((
                CognitoSessionInfo::from_claims(&refreshed_claims),
                fresh_id_token,
            ))
        }
        Err(_) => {
            // El refresh token ya venció o fue revocado: la sesión de
            // Cognito se da por perdida (sin tocar storage.json ni la base
            // del negocio, que siguen intactos). El usuario la va a ver pedir
            // de nuevo la próxima vez que la necesite.
            clear_tokens();
            let _ = clear_marker(config_dir);
            None
        }
    }
}

/// Se llama una sola vez, desde `lib.rs::setup`, **después** de que la base
/// de datos del negocio ya está abierta — un problema acá nunca debe impedir
/// que la app abra. Devuelve `None` si no hay sesión establecida o si no se
/// pudo recuperar (keychain vacío/corrupto, refresh token vencido), en cuyo
/// caso el frontend va a mostrar el gate de Cognito de nuevo.
pub fn resolve_session_on_startup(config_dir: &Path) -> Option<CognitoSessionInfo> {
    tauri::async_runtime::block_on(resolve_or_refresh(config_dir)).map(|(session, _)| session)
}

/// Pide un `id_token` vigente para una llamada puntual a `vendor::` (listar
/// negocios, conectar, etc.), refrescando en silencio si hace falta.
/// `AppError::Unauthorized` significa "no hay sesión usable" — el frontend
/// debe mostrar el login de la cuenta de Aura Studio de nuevo, igual que
/// cualquier otro `Unauthorized` ya existente (`AppState::require_user`).
pub async fn ensure_fresh_id_token(config_dir: &Path) -> AppResult<String> {
    resolve_or_refresh(config_dir)
        .await
        .map(|(_, id_token)| id_token)
        .ok_or(AppError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("aura-cognito-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn marker_round_trips_and_clears() {
        let dir = temp_dir();
        assert!(load_marker(&dir).is_none());

        let marker = CognitoInstallationMarker {
            established: true,
            username: "ana".to_string(),
            sub: "abc-123".to_string(),
            last_login_at: now_iso(),
        };
        save_marker(&dir, &marker).expect("debe poder escribir cognito.json");
        let loaded = load_marker(&dir).expect("debe poder leer cognito.json");
        assert_eq!(loaded.established, marker.established);
        assert_eq!(loaded.sub, marker.sub);

        clear_marker(&dir).expect("debe poder borrar cognito.json");
        assert!(load_marker(&dir).is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn decode_jwt_claims_reads_sub_exp_email_and_username() {
        let payload = URL_SAFE_NO_PAD.encode(
            br#"{"sub":"abc-123","exp":9999999999,"email":"a@b.com","cognito:username":"ana"}"#,
        );
        let token = format!("header.{payload}.signature");

        let claims = decode_jwt_claims(&token).expect("debe decodificar el payload");

        assert_eq!(claims.sub, "abc-123");
        assert_eq!(claims.exp, 9999999999);
        assert_eq!(claims.email.as_deref(), Some("a@b.com"));
        assert_eq!(claims.cognito_username.as_deref(), Some("ana"));
    }

    #[test]
    fn ensure_fresh_id_token_without_session_is_unauthorized() {
        let dir = temp_dir();
        let result = tauri::async_runtime::block_on(ensure_fresh_id_token(&dir));
        assert!(matches!(result, Err(AppError::Unauthorized)));
    }
}
