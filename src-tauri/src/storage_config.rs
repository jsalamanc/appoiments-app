//! Configuración del modo de almacenamiento de datos (local / PostgreSQL externo).
//!
//! `storage.json`, en el directorio de configuración de la app, guarda los datos
//! de conexión no sensibles (host, puerto, base, usuario, sslmode). La contraseña
//! nunca se escribe ahí: vive en el almacén de credenciales del sistema operativo
//! vía el crate `keyring`.
//!
//! Este módulo es la base del issue #9 (Fase 0): define el formato de
//! configuración y el acceso a credenciales. Todavía no está conectado a
//! `AppState`/`lib.rs` — esa conexión llega en la Fase 1, para no cambiar el
//! comportamiento del modo local mientras tanto.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::postgres::PgSslMode;

const CONFIG_FILE_NAME: &str = "storage.json";
const KEYRING_SERVICE: &str = "com.aurastudio.storage";

/// Modo de almacenamiento de datos activo para esta instalación.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum StorageMode {
    /// SQLite embebida (comportamiento actual, sin cambios).
    #[default]
    Local,
    /// Servidor PostgreSQL externo, auto-hospedado por el usuario.
    Postgres(PostgresConnectionConfig),
}

/// Datos de conexión no sensibles a un PostgreSQL externo. La contraseña se
/// maneja por separado vía [`set_password`]/[`get_password`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostgresConnectionConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    #[serde(default = "default_sslmode")]
    pub sslmode: String,
}

fn default_sslmode() -> String {
    "prefer".to_string()
}

impl PostgresConnectionConfig {
    /// Identificador estable de esta conexión, usado como cuenta en el keyring
    /// del sistema operativo. Solo hay una conexión externa activa a la vez,
    /// pero incluir sus datos evita reusar un secreto viejo si el usuario
    /// reconfigura la conexión sin borrar la anterior explícitamente.
    fn keyring_account(&self) -> String {
        format!(
            "{}@{}:{}/{}",
            self.username, self.host, self.port, self.database
        )
    }

    /// Arma las opciones de conexión de `sqlx` a partir de esta configuración
    /// y la contraseña (obtenida aparte del keyring). Se usa el builder de
    /// `PgConnectOptions` en vez de construir una URI a mano para no tener que
    /// lidiar con el percent-encoding de caracteres especiales en usuario,
    /// contraseña o nombre de base.
    pub fn connect_options(&self, password: &str) -> sqlx::postgres::PgConnectOptions {
        let ssl_mode = PgSslMode::from_str(&self.sslmode).unwrap_or_default();
        sqlx::postgres::PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(password)
            .database(&self.database)
            .ssl_mode(ssl_mode)
    }
}

fn config_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CONFIG_FILE_NAME)
}

/// Carga el modo de almacenamiento configurado. Si el archivo no existe
/// (instalación nueva, o una instalación previa a este cambio) o no se puede
/// leer/parsear, el modo por defecto es Local — nunca falla el arranque de la
/// app por un `storage.json` ausente o corrupto.
pub fn load(config_dir: &Path) -> StorageMode {
    fs::read_to_string(config_path(config_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(config_dir: &Path, mode: &StorageMode) -> std::io::Result<()> {
    fs::create_dir_all(config_dir)?;
    let raw = serde_json::to_string_pretty(mode).expect("StorageMode siempre serializa a JSON");
    fs::write(config_path(config_dir), raw)
}

/// Guarda la contraseña de conexión en el almacén de credenciales del sistema
/// operativo (Keychain en macOS, Credential Manager en Windows, Secret Service
/// en Linux).
pub fn set_password(cfg: &PostgresConnectionConfig, password: &str) -> keyring::Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, &cfg.keyring_account())?.set_password(password)
}

pub fn get_password(cfg: &PostgresConnectionConfig) -> keyring::Result<String> {
    keyring::Entry::new(KEYRING_SERVICE, &cfg.keyring_account())?.get_password()
}

pub fn delete_password(cfg: &PostgresConnectionConfig) -> keyring::Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, &cfg.keyring_account())?.delete_credential()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_defaults_to_local_when_missing() {
        let dir = std::env::temp_dir().join(format!("aura-storage-test-{}", uuid::Uuid::new_v4()));
        assert_eq!(load(&dir), StorageMode::Local);
    }

    #[test]
    fn save_then_load_roundtrips_postgres_mode() {
        let dir = std::env::temp_dir().join(format!("aura-storage-test-{}", uuid::Uuid::new_v4()));
        let mode = StorageMode::Postgres(PostgresConnectionConfig {
            host: "db.example.com".to_string(),
            port: 5432,
            database: "aura".to_string(),
            username: "aura_owner".to_string(),
            sslmode: "require".to_string(),
        });

        save(&dir, &mode).expect("debe poder escribir storage.json");
        assert_eq!(load(&dir), mode);

        fs::remove_dir_all(&dir).ok();
    }
}
