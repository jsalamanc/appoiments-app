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

/// Mapeo de errores de PostgreSQL (vía `sqlx`) a `AppError`, espejo de la
/// conversión de `rusqlite::Error` de arriba: mismos mensajes para el
/// usuario, pero leyendo el código SQLSTATE (`23505` = unicidad, `23503` =
/// llave foránea) y el nombre de la restricción en vez de parsear el texto
/// del mensaje de SQLite.
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        if matches!(err, sqlx::Error::RowNotFound) {
            return AppError::NotFound;
        }
        if let sqlx::Error::Database(ref db_err) = err {
            match db_err.code().as_deref() {
                Some("23505") => {
                    let msg = db_err.message();
                    if msg.contains("document") {
                        return AppError::conflict("Ya existe un cliente con ese documento");
                    }
                    if msg.contains("email") {
                        return AppError::conflict("Ya existe un cliente con ese correo");
                    }
                    if msg.contains("username") {
                        return AppError::conflict(
                            "Ya existe un usuario con ese nombre de usuario",
                        );
                    }
                    return AppError::conflict("Ya existe un registro con ese valor único");
                }
                Some("23503") => {
                    return AppError::conflict(
                        "No se puede completar la operación: hay registros que dependen de este elemento",
                    );
                }
                _ => {}
            }
        }
        AppError::Internal(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
