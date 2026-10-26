//! Configuración del bucket de almacenamiento en la nube (S3-compatible) para
//! las imágenes del catálogo, obligatorio cuando el modo de almacenamiento es
//! "Base de datos propia" (ver [`crate::storage_config`]).
//!
//! Mismo patrón que `storage_config.rs`: los datos no sensibles viven en un
//! archivo JSON en el directorio de configuración de la app (`bucket.json`,
//! separado de `storage.json` para no tocar el formato ya probado del modo
//! Postgres), y el secreto (Secret Access Key) vive en el almacén de
//! credenciales del sistema operativo vía el crate `keyring` — nunca en disco
//! en texto plano.
//!
//! El diseño es "un solo backend S3-compatible": AWS S3, Cloudflare R2,
//! Backblaze B2, DigitalOcean Spaces y MinIO hablan la misma API, así que
//! `provider` es solo un preset de UI (autocompletar endpoint/región) — el
//! código de subida/descarga (`crate::util::bucket`) es el mismo para todos.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const CONFIG_FILE_NAME: &str = "bucket.json";
const KEYRING_SERVICE: &str = "com.aurastudio.storage";

/// Preset de proveedor, solo para autocompletar endpoint/región en la UI — no
/// cambia el código de cliente, que es el mismo para cualquiera (API S3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BucketProvider {
    AwsS3,
    CloudflareR2,
    BackblazeB2,
    DigitalOceanSpaces,
    Custom,
}

/// Datos de conexión no sensibles al bucket. El Secret Access Key se maneja
/// por separado vía [`set_bucket_secret`]/[`get_bucket_secret`].
///
/// `camelCase` (a diferencia de `PostgresConnectionConfig`, cuyos campos ya
/// son de una sola palabra) para que el frontend reciba/envíe `accessKeyId`/
/// `pathStyle` en vez de los nombres `snake_case` crudos de Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketConfig {
    pub provider: BucketProvider,
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    #[serde(default)]
    pub prefix: Option<String>,
    pub access_key_id: String,
    #[serde(default)]
    pub path_style: bool,
}

impl BucketConfig {
    /// Identificador estable de este bucket, usado como cuenta en el keyring
    /// del sistema operativo (mismo criterio que
    /// `PostgresConnectionConfig::keyring_account`).
    fn keyring_account(&self) -> String {
        format!("bucket:{}@{}", self.bucket, self.endpoint)
    }

    /// Key del objeto en el bucket para un `photo_path` dado. Deliberadamente
    /// el mismo string relativo que ya se usa como ruta en disco
    /// (`images/<folder>/<archivo>`) — ver `crate::util::photos` — con el
    /// prefijo opcional antepuesto si se configuró uno, para no mezclarse con
    /// otros usos del mismo bucket.
    pub fn object_key(&self, photo_path: &str) -> String {
        match &self.prefix {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}/{photo_path}"),
            _ => photo_path.to_string(),
        }
    }
}

fn config_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CONFIG_FILE_NAME)
}

/// Carga la configuración del bucket guardada, si hay una. `None` si no se ha
/// configurado nunca o el archivo no se puede leer/parsear — igual que
/// `storage_config::load`, nunca falla el arranque de la app por esto.
pub fn load_bucket(config_dir: &Path) -> Option<BucketConfig> {
    fs::read_to_string(config_path(config_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

pub fn save_bucket(config_dir: &Path, config: &BucketConfig) -> std::io::Result<()> {
    fs::create_dir_all(config_dir)?;
    let raw = serde_json::to_string_pretty(config).expect("BucketConfig siempre serializa a JSON");
    fs::write(config_path(config_dir), raw)
}

/// Guarda el Secret Access Key en el almacén de credenciales del sistema
/// operativo (Keychain en macOS, Credential Manager en Windows, Secret
/// Service en Linux) — nunca se escribe en `bucket.json`.
pub fn set_bucket_secret(cfg: &BucketConfig, secret: &str) -> keyring::Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, &cfg.keyring_account())?.set_password(secret)
}

pub fn get_bucket_secret(cfg: &BucketConfig) -> keyring::Result<String> {
    keyring::Entry::new(KEYRING_SERVICE, &cfg.keyring_account())?.get_password()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> BucketConfig {
        BucketConfig {
            provider: BucketProvider::AwsS3,
            endpoint: "https://s3.us-east-1.amazonaws.com".to_string(),
            region: "us-east-1".to_string(),
            bucket: "aura-studio-fotos".to_string(),
            prefix: None,
            access_key_id: "AKIAEXAMPLE".to_string(),
            path_style: false,
        }
    }

    #[test]
    fn load_returns_none_when_missing() {
        let dir = std::env::temp_dir().join(format!("aura-bucket-test-{}", uuid::Uuid::new_v4()));
        assert_eq!(load_bucket(&dir), None);
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = std::env::temp_dir().join(format!("aura-bucket-test-{}", uuid::Uuid::new_v4()));
        let cfg = sample_config();
        save_bucket(&dir, &cfg).expect("debe poder escribir bucket.json");
        assert_eq!(load_bucket(&dir), Some(cfg));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn object_key_uses_prefix_when_present() {
        let mut cfg = sample_config();
        assert_eq!(cfg.object_key("images/products/a.jpg"), "images/products/a.jpg");
        cfg.prefix = Some("catalogo".to_string());
        assert_eq!(cfg.object_key("images/products/a.jpg"), "catalogo/images/products/a.jpg");
    }
}
