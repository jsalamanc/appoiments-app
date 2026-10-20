use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("No encontrado")]
    NotFound,
    #[error("No autorizado")]
    Unauthorized,
    #[error("{0}")]
    Conflict(String),
    #[error("Error interno: {0}")]
    Internal(String),
}

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(e, msg) = &err {
            if e.code == rusqlite::ErrorCode::ConstraintViolation {
                let text = msg.clone().unwrap_or_default();
                if text.contains("UNIQUE") {
                    if text.contains("document") {
                        return AppError::conflict("Ya existe un cliente con ese documento");
                    }
                    if text.contains("email") {
                        return AppError::conflict("Ya existe un cliente con ese correo");
                    }
                    if text.contains("username") {
                        return AppError::conflict("Ya existe un usuario con ese nombre de usuario");
                    }
                    return AppError::conflict("Ya existe un registro con ese valor único");
                }
                if text.contains("FOREIGN KEY") {
                    return AppError::conflict(
                        "No se puede completar la operación: hay registros que dependen de este elemento",
                    );
                }
            }
        }
        if err == rusqlite::Error::QueryReturnedNoRows {
            return AppError::NotFound;
        }
        AppError::Internal(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
