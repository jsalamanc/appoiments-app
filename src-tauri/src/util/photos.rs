use std::path::{Path, PathBuf};

use crate::db::new_id;
use crate::error::{AppError, AppResult};

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
