use tauri::{AppHandle, Manager, State};

use crate::cognito;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::vendor::{self, VendorBusiness, VendorClient, VendorConnectPreview};

fn config_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError::Internal(format!("no se pudo resolver el directorio de configuración: {e}")))
}

#[tauri::command]
pub async fn vendor_list_businesses(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Vec<VendorBusiness>> {
    state.require_owner()?;
    let dir = config_dir(&app)?;
    let id_token = cognito::ensure_fresh_id_token(&dir).await?;
    VendorClient::new(id_token).list_businesses().await
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBusinessInput {
    pub business_name: String,
}

#[tauri::command]
pub async fn vendor_create_business(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateBusinessInput,
) -> AppResult<VendorBusiness> {
    state.require_owner()?;
    if input.business_name.trim().is_empty() {
        return Err(AppError::validation("El nombre del negocio es obligatorio"));
    }
    let dir = config_dir(&app)?;
    let id_token = cognito::ensure_fresh_id_token(&dir).await?;
    VendorClient::new(id_token)
        .create_business(input.business_name.trim())
        .await
}

/// Vista previa: pide las credenciales de conexión de un negocio sin
/// persistir nada acá (igual que `storage_test_connection` para Postgres) —
/// `storage_switch_mode` es quien de verdad conecta y guarda. Esta llamada sí
/// tiene un efecto del lado del servidor (vincula esta instalación al
/// negocio, o confirma que ya lo estaba), aunque ninguno local.
#[tauri::command]
pub async fn vendor_connect_business(
    app: AppHandle,
    state: State<'_, AppState>,
    business_id: String,
) -> AppResult<VendorConnectPreview> {
    state.require_owner()?;
    let dir = config_dir(&app)?;
    let id_token = cognito::ensure_fresh_id_token(&dir).await?;
    let installation_id = vendor::get_or_create_installation_id(&dir)
        .map_err(|e| AppError::Internal(format!("no se pudo preparar el id de instalación: {e}")))?;
    let outcome = VendorClient::new(id_token)
        .get_connection(&business_id, &installation_id)
        .await?;
    Ok(VendorConnectPreview::from(&outcome))
}

#[tauri::command]
pub async fn vendor_unlink_installation(
    app: AppHandle,
    state: State<'_, AppState>,
    business_id: String,
) -> AppResult<()> {
    state.require_owner()?;
    let dir = config_dir(&app)?;
    let id_token = cognito::ensure_fresh_id_token(&dir).await?;
    VendorClient::new(id_token)
        .unlink_installation(&business_id)
        .await
}
