use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::app_db::AppDb;
use crate::cognito::CognitoSessionInfo;
use crate::error::{AppError, AppResult};
use crate::models::user::UserPublic;

pub struct AppState {
    pub db: AppDb,
    pub current_user: Mutex<Option<UserPublic>>,
    /// Sesión de Cognito (identidad de quien administra la suscripción/
    /// negocio en la nube) — totalmente independiente de `current_user`
    /// (quién opera la app ahora mismo, rol `owner`/`staff`). Ver
    /// `crate::cognito`.
    pub cognito_session: Mutex<Option<CognitoSessionInfo>>,
    /// Cache en memoria del `id_token` vigente — nunca se persiste (ni en el
    /// keyring ni en disco): en Windows, el Credential Manager rechaza
    /// credenciales de más de 2560 bytes, y un `id_token` de Cognito con
    /// varios grupos/atributos personalizados puede superar ese límite. Se
    /// pierde al cerrar la app, lo cual está bien: `cognito::resolve_or_refresh`
    /// lo repone pidiendo uno nuevo con el `refresh_token` (ese sí persistido).
    pub cognito_id_token: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(db: Connection) -> Self {
        Self {
            db: AppDb::Sqlite(Arc::new(Mutex::new(db))),
            current_user: Mutex::new(None),
            cognito_session: Mutex::new(None),
            cognito_id_token: Mutex::new(None),
        }
    }

    pub fn new_postgres(pool: sqlx::PgPool) -> Self {
        Self {
            db: AppDb::Postgres(pool),
            current_user: Mutex::new(None),
            cognito_session: Mutex::new(None),
            cognito_id_token: Mutex::new(None),
        }
    }

    /// Exige que haya una sesión activa, de cualquier rol (`owner` o
    /// `staff`). Es el chequeo mínimo que debería tener casi cualquier
    /// comando operativo (catálogo, citas, clientes, ventas, inventario) —
    /// hasta ahora la mayoría de esos comandos no verificaban nada en
    /// absoluto (issue #2).
    pub fn require_user(&self) -> AppResult<UserPublic> {
        self.current_user
            .lock()
            .unwrap()
            .clone()
            .ok_or(AppError::Unauthorized)
    }

    /// Exige que haya una sesión activa con rol `owner`. Usado por los
    /// comandos de Ajustes/Reportes, que en el frontend ya están ocultos
    /// para staff (OwnerGate + sidebar), pero que también deben rechazarse
    /// aquí para que la restricción sea real y no solo de interfaz.
    pub fn require_owner(&self) -> AppResult<UserPublic> {
        let current = self.require_user()?;
        if current.role != "owner" {
            return Err(AppError::Unauthorized);
        }
        Ok(current)
    }
}
