use tauri::State;

use crate::audit::{self, AuditFilters, ListAuditResult};
use crate::error::AppResult;
use crate::state::AppState;

/// Consulta de la bitácora de auditoría. Solo el dueño: la bitácora incluye
/// montos, costos y acciones de todo el equipo.
#[tauri::command]
pub async fn list_audit_log(
    state: State<'_, AppState>,
    filters: AuditFilters,
) -> AppResult<ListAuditResult> {
    state.require_owner()?;
    audit::list(&state.db, filters).await
}
