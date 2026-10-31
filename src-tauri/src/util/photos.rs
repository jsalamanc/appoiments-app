use std::path::{Path, PathBuf};

use crate::app_db::AppDb;
use crate::bucket_config;
use crate::db::new_id;
use crate::error::{AppError, AppResult};
use crate::util::bucket::BucketClient;

const ALLOWED_PHOTO_FOLDERS: [&str; 3] = ["products", "services", "packages"];

/// Valida que `photo_path` tenga la forma exacta `images/<folder>/<nombre>.<ext>`
/// generada por [`import_photo`], antes de usarlo para leer/escribir/borrar en
/// disco o en el bucket (`resolve_photo_cache`, `discard_photo`). Sin esto, un
/// `photo_path` recibido por IPC con `..` o una ruta absoluta podría escapar
/// de `<data_dir>/images/`.
pub fn validate_photo_path(photo_path: &str) -> AppResult<()> {
    let invalid = || AppError::validation("Ruta de imagen inválida");

    let parts: Vec<&str> = photo_path.split('/').collect();
    let [prefix, folder, file_name] = parts[..] else {
        return Err(invalid());
    };
    if prefix != "images" || !ALLOWED_PHOTO_FOLDERS.contains(&folder) {
        return Err(invalid());
    }

    let (stem, ext) = file_name.rsplit_once('.').ok_or_else(invalid)?;
    let stem_ok = !stem.is_empty() && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let ext_ok = !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric());
    if !stem_ok || !ext_ok {
        return Err(invalid());
    }

    Ok(())
}

/// Copia `source_file_path` (ruta absoluta elegida por el usuario en un file
/// picker, ver `@tauri-apps/plugin-dialog`) dentro de
/// `<data_dir>/images/<folder>/<uuid>.<ext>` y devuelve la ruta relativa (a
/// partir de `data_dir`) para guardar en la columna `photo_path`.
///
/// Reemplaza a Supabase Storage: ya no hay URLs públicas, solo rutas de
/// archivo locales que el frontend resuelve con `convertFileSrc` de
/// `@tauri-apps/api/core` (uniendo `appDataDir()` + `photo_path`).
pub fn import_photo(data_dir: &Path, folder: &str, source_file_path: &str) -> AppResult<String> {
    let source = PathBuf::from(source_file_path);
    if !source.is_file() {
        return Err(AppError::validation("El archivo seleccionado no existe"));
    }

    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();

    let dest_dir = data_dir.join("images").join(folder);
    std::fs::create_dir_all(&dest_dir).map_err(|e| {
        AppError::Internal(format!("no se pudo crear el directorio de imágenes: {e}"))
    })?;

    let file_name = format!("{}.{}", new_id(), ext);
    let dest_path = dest_dir.join(&file_name);

    std::fs::copy(&source, &dest_path)
        .map_err(|e| AppError::Internal(format!("no se pudo copiar la imagen: {e}")))?;

    Ok(format!("images/{folder}/{file_name}"))
}

/// Igual que [`import_photo`], y además sube la imagen al bucket configurado
/// cuando el modo de almacenamiento activo es PostgreSQL (el bucket es
/// obligatorio en ese modo, ver `commands::storage::storage_switch_mode`).
///
/// El disco local sigue siendo el destino de la copia en ambos modos: en
/// modo Postgres actúa como caché de escritura, con la misma ruta relativa
/// usada también como key del objeto en el bucket (ver
/// `BucketConfig::object_key`) — así `photo_path` es idéntico sin importar el
/// modo de almacenamiento (ver `resolve_photo_cache` para la lectura).
pub async fn import_and_sync_photo(
    data_dir: &Path,
    config_dir: &Path,
    db: &AppDb,
    folder: &str,
    source_file_path: &str,
) -> AppResult<String> {
    let photo_path = import_photo(data_dir, folder, source_file_path)?;

    if matches!(db, AppDb::Postgres(_)) {
        // Si la subida al bucket falla por cualquier motivo, la copia local
        // recién creada no debe sobrevivir: nadie va a recibir este
        // `photo_path` (el comando devuelve el error, no el path), así que
        // dejarla sería un huérfano inmediato — mismo espíritu que
        // `cleanup_old_photo`, pero acá limpiando el lado local que sí se
        // alcanzó a escribir.
        if let Err(e) = upload_to_bucket(data_dir, config_dir, &photo_path).await {
            let _ = std::fs::remove_file(data_dir.join(&photo_path));
            return Err(e);
        }
    }

    Ok(photo_path)
}

async fn upload_to_bucket(data_dir: &Path, config_dir: &Path, photo_path: &str) -> AppResult<()> {
    let cfg = bucket_config::load_bucket(config_dir).ok_or_else(|| {
        AppError::Internal(
            "no hay un bucket configurado; el modo \"Base de datos propia\" requiere uno".to_string(),
        )
    })?;
    let secret = bucket_config::get_bucket_secret(&cfg)
        .map_err(|e| AppError::Internal(format!("no se pudo leer la credencial del bucket: {e}")))?;
    let client = BucketClient::new(&cfg, &secret)?;
    let bytes = std::fs::read(data_dir.join(photo_path))
        .map_err(|e| AppError::Internal(format!("no se pudo leer la imagen recién copiada: {e}")))?;
    client.put_object(&cfg.object_key(photo_path), bytes).await
}

/// Borra `photo_path` del disco local y, en modo Postgres, también del
/// bucket — pensada para llamarse después de que un producto/servicio/
/// paquete se borró o reemplazó su foto, para no dejar imágenes huérfanas
/// (issue #18).
///
/// Deliberadamente **best-effort**: el registro ya se borró/actualizó con
/// éxito en la base de datos cuando esto se invoca, así que un fallo acá
/// (archivo ya no existe, bucket momentáneamente inalcanzable, credencial
/// vencida) no debe revertir ni bloquear esa operación — como mucho deja la
/// misma clase de huérfano que ya existía antes de este cleanup, nunca algo
/// peor. Por eso no devuelve `AppResult`: los llamadores no tienen nada
/// razonable que hacer con un error de limpieza.
pub async fn cleanup_old_photo(
    data_dir: &Path,
    config_dir: &Path,
    db: &AppDb,
    photo_path: Option<&str>,
) {
    let Some(photo_path) = photo_path else {
        return;
    };

    let _ = std::fs::remove_file(data_dir.join(photo_path));

    if matches!(db, AppDb::Postgres(_)) {
        if let Some(cfg) = bucket_config::load_bucket(config_dir) {
            if let Ok(secret) = bucket_config::get_bucket_secret(&cfg) {
                if let Ok(client) = BucketClient::new(&cfg, &secret) {
                    let _ = client.delete_object(&cfg.object_key(photo_path)).await;
                }
            }
        }
    }
}
