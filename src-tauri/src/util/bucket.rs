//! Cliente mínimo para subir/descargar/borrar objetos en un bucket
//! S3-compatible (AWS S3, Cloudflare R2, Backblaze B2, DigitalOcean Spaces,
//! MinIO, etc.) — ver `crate::bucket_config` para la configuración.
//!
//! Usa `rusty-s3` solo para firmar las URLs (Sans-IO, sin cliente HTTP
//! propio) y `reqwest` para hacer las peticiones — deliberadamente liviano en
//! vez del SDK oficial de AWS, ya que el objetivo es hablar con cualquier
//! endpoint S3-compatible, no solo AWS.
//!
//! No se setea `Content-Type` al subir: el contenido de estos objetos nunca
//! se sirve directo a un navegador (ver la decisión de diseño de "disco local
//! como caché" en `crate::util::photos::import_and_sync_photo` y el comando
//! `resolve_photo_cache`), así que no hace falta.

use std::time::Duration;

use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};

use crate::bucket_config::BucketConfig;
use crate::error::{AppError, AppResult};

/// Las URLs firmadas se usan de inmediato, apenas generadas — una expiración
/// corta reduce la ventana en la que una URL filtrada (ej. en un log) sirve
/// para algo.
const SIGN_EXPIRY: Duration = Duration::from_secs(60);

/// Key de prueba usada por `test_connection`. Con prefijo `__` para no
/// mezclarse por accidente con una key real de `images/<folder>/...`.
const TEST_OBJECT_KEY: &str = "__aura_storage_test__";

pub struct BucketClient {
    bucket: Bucket,
    credentials: Credentials,
    http: reqwest::Client,
}

impl BucketClient {
    pub fn new(cfg: &BucketConfig, secret: &str) -> AppResult<Self> {
        let endpoint = cfg
            .endpoint
            .parse()
            .map_err(|e| AppError::validation(format!("Endpoint de bucket inválido: {e}")))?;
        let url_style = if cfg.path_style {
            UrlStyle::Path
        } else {
            UrlStyle::VirtualHost
        };
        let bucket = Bucket::new(endpoint, url_style, cfg.bucket.clone(), cfg.region.clone())
            .map_err(|e| AppError::validation(format!("Configuración de bucket inválida: {e}")))?;
        let credentials = Credentials::new(cfg.access_key_id.clone(), secret.to_string());

        Ok(Self {
            bucket,
            credentials,
            http: reqwest::Client::new(),
        })
    }

    pub async fn put_object(&self, key: &str, bytes: Vec<u8>) -> AppResult<()> {
        let url = self
            .bucket
            .put_object(Some(&self.credentials), key)
            .sign(SIGN_EXPIRY);
        let response = self
            .http
            .put(url)
            .body(bytes)
            .send()
            .await
            .map_err(|e| AppError::validation(format!("No se pudo subir al bucket: {e}")))?;
        ensure_success(response, "subir la imagen al bucket").await?;
        Ok(())
    }

    pub async fn get_object(&self, key: &str) -> AppResult<Vec<u8>> {
        let url = self
            .bucket
            .get_object(Some(&self.credentials), key)
            .sign(SIGN_EXPIRY);
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::validation(format!("No se pudo descargar del bucket: {e}")))?;
        let response = ensure_success(response, "descargar la imagen del bucket").await?;
        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| AppError::Internal(format!("respuesta del bucket inválida: {e}")))
    }

    pub async fn delete_object(&self, key: &str) -> AppResult<()> {
        let url = self
            .bucket
            .delete_object(Some(&self.credentials), key)
            .sign(SIGN_EXPIRY);
        let response = self
            .http
            .delete(url)
            .send()
            .await
            .map_err(|e| AppError::validation(format!("No se pudo borrar del bucket: {e}")))?;
        ensure_success(response, "borrar el objeto de prueba del bucket").await?;
        Ok(())
    }

    /// Sube, descarga y borra un objeto de prueba pequeño para confirmar que
    /// las credenciales, el bucket y el endpoint sirven de verdad — mismo
    /// espíritu que `storage_test_connection` (que hace un `SELECT 1` real
    /// en vez de solo "parsear" los datos de conexión).
    pub async fn test_connection(&self) -> AppResult<()> {
        let payload = b"aura-studio-test".to_vec();
        self.put_object(TEST_OBJECT_KEY, payload.clone()).await?;
        let downloaded = self.get_object(TEST_OBJECT_KEY).await;
        // Limpieza best-effort: si el borrado falla, no es motivo para
        // reportar la prueba como fallida (el bucket ya demostró que sirve).
        let _ = self.delete_object(TEST_OBJECT_KEY).await;

        match downloaded {
            Ok(bytes) if bytes == payload => Ok(()),
            Ok(_) => Err(AppError::validation(
                "El bucket devolvió un contenido distinto al que se subió",
            )),
            Err(e) => Err(e),
        }
    }
}

async fn ensure_success(response: reqwest::Response, action: &str) -> AppResult<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(AppError::validation(format!(
        "No se pudo {action} (HTTP {status}): {body}"
    )))
}
