use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::app_db::AppDb;
use crate::error::{AppError, AppResult};
use crate::models::user::UserPublic;

pub struct AppState {
    pub db: AppDb,
    pub current_user: Mutex<Option<UserPublic>>,
}

impl AppState {
    pub fn new(db: Connection) -> Self {
        Self {
            db: AppDb::Sqlite(Arc::new(Mutex::new(db))),
            current_user: Mutex::new(None),
        }
    }

    /// Exige que haya una sesión activa con rol `owner`. Usado por los
    /// comandos de Ajustes/Reportes, que en el frontend ya están ocultos
    /// para staff (OwnerGate + sidebar), pero que también deben rechazarse
    /// aquí para que la restricción sea real y no solo de interfaz.
    pub fn require_owner(&self) -> AppResult<UserPublic> {
        let current = self
            .current_user
            .lock()
            .unwrap()
            .clone()
            .ok_or(AppError::Unauthorized)?;
        if current.role != "owner" {
            return Err(AppError::Unauthorized);
        }
        Ok(current)
    }
}
