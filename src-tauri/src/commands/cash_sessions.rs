//! Apertura y cierre de caja (issue #2, Fase 5).

use tauri::State;

use crate::app_db::AppDb;
use crate::error::AppResult;
use crate::models::cash_session::CashSession;
use crate::repositories::cash_sessions as repo;
use crate::state::AppState;

#[tauri::command]
pub async fn get_current_cash_session(
    state: State<'_, AppState>,
) -> AppResult<Option<CashSession>> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(repo::get_current).await,
        AppDb::Postgres(pool) => repo::postgres::get_current(pool).await,
    }
}

#[tauri::command]
pub async fn list_cash_sessions(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<CashSession>> {
    state.require_user()?;
    let limit = limit.unwrap_or(30).clamp(1, 200);
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::list(conn, limit)).await,
        AppDb::Postgres(pool) => repo::postgres::list(pool, limit).await,
    }
}

#[tauri::command]
pub async fn open_cash_session(
    state: State<'_, AppState>,
    opening_amount: f64,
) -> AppResult<CashSession> {
    let user_id = state.require_user()?.id;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::open(conn, Some(user_id), opening_amount))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::open(pool, Some(user_id), opening_amount).await,
    }
}

#[tauri::command]
pub async fn close_cash_session(
    state: State<'_, AppState>,
    id: String,
    counted_amount: f64,
    notes: Option<String>,
) -> AppResult<CashSession> {
    let user_id = state.require_user()?.id;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::close(conn, id, Some(user_id), counted_amount, notes))
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::close(pool, id, Some(user_id), counted_amount, notes).await
        }
    }
}
