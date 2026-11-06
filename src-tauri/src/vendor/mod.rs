//! Cliente mínimo contra la API de negocio ("vendor") del backend en la nube
//! (`appoiments-cloud-backend`): listar/crear negocios, conectar a la base de
//! datos gestionada, y manejar el vínculo con esta instalación. Mismo
//! espíritu liviano que `cognito::CognitoClient` — sin SDK, solo `reqwest` +
//! JSON, un `Authorization: Bearer <id_token>` en vez de las cabeceras
//! `X-Amz-Target` que usa Cognito.
//!
//! El backend ya valida que el `id_token` sea el de Cognito (el API Gateway
//! de ese repo exige audience = app client id, que solo lleva el ID token,
//! nunca el access token) — acá no hace falta volver a validar nada, solo
//! mandarlo.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

// Terraform output `api_endpoint` de
// `appoiments-cloud-backend/infra/environments/{dev,prod}`. Público (lo
// contacta cualquier instalación de escritorio) — no es secreto, pero igual
// va por variable de entorno en tiempo de compilación, mismo criterio y
// mismo mecanismo que `cognito::USER_POOL_ID`/`APP_CLIENT_ID` (ver
// build.rs y el comentario largo en cognito/mod.rs).
const VENDOR_API_BASE_URL: &str = env!("VENDOR_API_BASE_URL");

const INSTALLATION_ID_FILE_NAME: &str = "installation-id.txt";

/// Id opaco y estable de esta instalación de escritorio, generado una sola
/// vez y reutilizado en cada llamada a la API de negocio (`installationId`) —
/// el backend no genera ids de dispositivo, le toca al cliente. No es
/// sensible, es solo un correlacionador, por eso vive como archivo plano
/// (no en el keychain) junto a `storage.json`/`cognito.json`.
pub fn get_or_create_installation_id(config_dir: &Path) -> std::io::Result<String> {
    let path = config_dir.join(INSTALLATION_ID_FILE_NAME);
    if let Ok(existing) = fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    fs::create_dir_all(config_dir)?;
    fs::write(&path, &id)?;
    Ok(id)
}

// --- Shapes del backend --------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorBusiness {
    pub id: String,
    pub business_name: String,
    pub subscription_status: Option<String>,
    pub has_database: bool,
    pub installation_id: Option<String>,
}

// Deliberadamente NO deriva `Serialize`: trae la contraseña en texto plano y
// nunca debe cruzar el límite de IPC hacia el frontend/webview. La única
// lectora de este struct es `storage_switch_mode` (que la pasa directo a
// `open_postgres`/`storage_config::set_password`, nunca al JS). El comando
// de vista previa (`vendor_connect_business`) la descarta antes de responder
// -- ver `VendorConnectPreview`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorConnection {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    pub ssl_mode: String,
}

/// Resultado interno de `get_connection`: el caso "ya vinculado a otra
/// instalación" (HTTP 409) es un desenlace esperado del flujo, no una falla.
/// Nunca se serializa tal cual hacia el frontend (traería la contraseña) --
/// `commands::vendor::vendor_connect_business` lo mapea a
/// `VendorConnectPreview` antes de responder.
#[derive(Debug, Clone)]
pub enum VendorConnectOutcome {
    Connected(VendorConnection),
    LinkedElsewhere,
}

/// Versión segura para IPC de `VendorConnectOutcome`, sin la contraseña ni el
/// resto de los datos de conexión -- el frontend solo necesita saber si
/// puede mostrar el diálogo de confirmación o el aviso de "ya vinculado en
/// otro equipo".
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum VendorConnectPreview {
    Connected,
    LinkedElsewhere,
}

impl From<&VendorConnectOutcome> for VendorConnectPreview {
    fn from(outcome: &VendorConnectOutcome) -> Self {
        match outcome {
            VendorConnectOutcome::Connected(_) => VendorConnectPreview::Connected,
            VendorConnectOutcome::LinkedElsewhere => VendorConnectPreview::LinkedElsewhere,
        }
    }
}

#[derive(Debug, Deserialize)]
struct VendorErrorBody {
    error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateBusinessBody<'a> {
    business_name: &'a str,
}

fn map_vendor_error(status: reqwest::StatusCode, error_code: &str) -> AppError {
    match error_code {
        "unauthorized" => AppError::Unauthorized,
        "business_not_found" | "not_linked" => AppError::NotFound,
        "missing_installation_id" | "invalid_request" => AppError::validation("Solicitud inválida"),
        "not_provisioned" => AppError::validation(
            "Este negocio todavía no tiene una base de datos lista. Intenta de nuevo en unos minutos.",
        ),
        _ => AppError::Internal(format!("La API de negocio respondió {status}: {error_code}")),
    }
}

pub struct VendorClient {
    http: reqwest::Client,
    id_token: String,
}

impl VendorClient {
    pub fn new(id_token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            id_token,
        }
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> AppResult<reqwest::RequestBuilder> {
        Ok(self
            .http
            .request(method, format!("{VENDOR_API_BASE_URL}{path}"))
            .header("Authorization", format!("Bearer {}", self.id_token)))
    }

    /// Manda la petición y, si la respuesta no es 2xx, la mapea a `AppError`.
    /// Devuelve el `reqwest::Response` crudo en éxito para que cada método
    /// decida cómo parsear el cuerpo (o, en el caso de `get_connection`, para
    /// leer el código de error específico antes de mapear).
    async fn send(&self, builder: reqwest::RequestBuilder) -> Result<reqwest::Response, (reqwest::StatusCode, String)> {
        let response = builder.send().await.map_err(|e| {
            (
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                format!("No se pudo contactar la API de negocio: {e}"),
            )
        })?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        let code = serde_json::from_str::<VendorErrorBody>(&body_text)
            .map(|b| b.error)
            .unwrap_or_else(|_| body_text);
        Err((status, code))
    }

    pub async fn list_businesses(&self) -> AppResult<Vec<VendorBusiness>> {
        let builder = self.request(reqwest::Method::GET, "/vendor/businesses").await?;
        let response = self
            .send(builder)
            .await
            .map_err(|(status, code)| map_vendor_error(status, &code))?;
        response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Respuesta de negocios inválida: {e}")))
    }

    pub async fn create_business(&self, business_name: &str) -> AppResult<VendorBusiness> {
        let builder = self
            .request(reqwest::Method::POST, "/vendor/businesses")
            .await?;
        let response = self
            .send(builder.json(&CreateBusinessBody { business_name }))
            .await
            .map_err(|(status, code)| map_vendor_error(status, &code))?;
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Created {
            id: String,
            business_name: String,
        }
        let created: Created = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Respuesta de crear negocio inválida: {e}")))?;
        Ok(VendorBusiness {
            id: created.id,
            business_name: created.business_name,
            subscription_status: None,
            has_database: false,
            installation_id: None,
        })
    }

    pub async fn get_connection(
        &self,
        business_id: &str,
        installation_id: &str,
    ) -> AppResult<VendorConnectOutcome> {
        let path = format!("/vendor/businesses/{business_id}/connection?installationId={installation_id}");
        let builder = self.request(reqwest::Method::GET, &path).await?;
        match self.send(builder).await {
            Ok(response) => {
                let connection: VendorConnection = response.json().await.map_err(|e| {
                    AppError::Internal(format!("Respuesta de conexión inválida: {e}"))
                })?;
                Ok(VendorConnectOutcome::Connected(connection))
            }
            Err((_, code)) if code == "installation_linked_elsewhere" => {
                Ok(VendorConnectOutcome::LinkedElsewhere)
            }
            Err((status, code)) => Err(map_vendor_error(status, &code)),
        }
    }

    pub async fn unlink_installation(&self, business_id: &str) -> AppResult<()> {
        let path = format!("/vendor/businesses/{business_id}/unlink-installation");
        let builder = self.request(reqwest::Method::POST, &path).await?;
        self.send(builder)
            .await
            .map_err(|(status, code)| map_vendor_error(status, &code))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_id_is_generated_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("aura-vendor-test-{}", uuid::Uuid::new_v4()));
        let first = get_or_create_installation_id(&dir).expect("debe poder crear el id");
        let second = get_or_create_installation_id(&dir).expect("debe poder releerlo");
        assert_eq!(first, second);
        fs::remove_dir_all(&dir).ok();
    }
}
