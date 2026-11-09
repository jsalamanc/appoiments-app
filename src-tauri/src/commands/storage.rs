use tauri::{AppHandle, Emitter, Manager, State};

use crate::app_db;
use crate::app_db::AppDb;
use crate::audit::{self, NewEntry};
use crate::bucket_config::{self, BucketConfig};
use crate::cognito;
use crate::error::{AppError, AppResult};
use crate::export;
use crate::migration::{self, MigrationProgressEvent, MigrationSummary};
use crate::state::AppState;
use crate::storage_config::{self, NubeConnectionConfig, PostgresConnectionConfig, StorageMode};
use crate::util::bucket::BucketClient;
use crate::vendor::{self, VendorClient, VendorConnectOutcome};

fn config_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError::Internal(format!("no se pudo resolver el directorio de configuración: {e}")))
}

fn data_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}")))
}

/// Callback de progreso para `migration::migrate_local_to_postgres` — emite
/// `storage-migration-progress` para que el frontend pueda cerrar el diálogo
/// de confirmación y mostrar un toast persistente mientras el usuario sigue
/// navegando (la migración puede tardar varios minutos en una cuenta con
/// mucho historial, al insertar fila por fila).
fn progress_emitter(app: &AppHandle) -> impl FnMut(usize, usize) + Send + '_ {
    move |done, total| {
        let _ = app.emit(
            "storage-migration-progress",
            MigrationProgressEvent { done, total },
        );
    }
}

/// Modo de almacenamiento activo, para que Ajustes sepa qué mostrar
/// seleccionado al cargar la página. No incluye la contraseña (vive aparte,
/// en el keychain del SO) — `StorageMode` ya está diseñado para serializarse
/// de forma segura.
#[tauri::command]
pub async fn storage_get_mode(app: AppHandle, state: State<'_, AppState>) -> AppResult<StorageMode> {
    state.require_owner()?;
    Ok(storage_config::load(&config_dir(&app)?))
}

/// Cambia el modo de almacenamiento activo. Para `nube`, se conecta y se
/// corren las migraciones de esquema contra el destino, y opcionalmente se
/// migran los datos locales (issue #11), incluidas las imágenes del catálogo
/// al bucket configurado (issue #18) — todo ANTES de persistir nada — si algo
/// falla, `storage.json`/`bucket.json` quedan exactamente como estaban (no
/// hace falta un "revert" explícito, el orden de las operaciones ya lo
/// garantiza). El frontend es responsable de reiniciar la app (`relaunch()`
/// del plugin `process`) después de un cambio exitoso: `AppState` ya quedó
/// construido contra el backend anterior y no se puede reemplazar en
/// caliente.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn storage_switch_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: String,
    migrate_data: Option<bool>,
    replace_data: Option<bool>,
    bucket_config: Option<BucketConfig>,
    bucket_secret: Option<String>,
    nube_business_id: Option<String>,
    nube_business_name: Option<String>,
) -> AppResult<Option<MigrationSummary>> {
    let mode_for_log = mode.clone();
    let audit_state = state.clone();
    let audit_actor = state.require_owner().ok();
    // Separado en una función interna solo por esto: `?` dentro de un
    // `match` no "sale" del `match`, sale de toda la función que lo
    // contiene, así que intentar envolver el `match` de abajo con un
    // `let result = match { ... }; log; result` directamente nunca llegaba
    // a loguear nada (todos los caminos de error reales usan `?`). Separar
    // la lógica a una función aparte sí permite capturar su `Result` acá.
    let result = storage_switch_mode_inner(
        app,
        state,
        mode,
        migrate_data,
        replace_data,
        bucket_config,
        bucket_secret,
        nube_business_id,
        nube_business_name,
    )
    .await;
    if let Err(e) = &result {
        eprintln!("[storage] switch_mode({mode_for_log}) failed: {e}");
    } else {
        // Se registra en la base que estaba activa hasta ahora: la nueva
        // solo se adopta al reiniciar la app.
        audit::record(
            &audit_state.db,
            audit_actor.as_ref(),
            NewEntry::event(
                "storage.switch_mode",
                None,
                format!("Cambió el almacenamiento de datos a «{mode_for_log}»"),
            ),
        )
        .await;
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn storage_switch_mode_inner(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: String,
    migrate_data: Option<bool>,
    replace_data: Option<bool>,
    bucket_config: Option<BucketConfig>,
    bucket_secret: Option<String>,
    nube_business_id: Option<String>,
    nube_business_name: Option<String>,
) -> AppResult<Option<MigrationSummary>> {
    state.require_owner()?;
    let dir = config_dir(&app)?;

    // Si esta instalación venía conectada a un negocio de Nube y el cambio la
    // aleja de ese negocio (a Local, o a otro negocio de Nube), se desvincula
    // del lado del servidor -- best-effort: sin
    // internet, o si la sesión de la cuenta venció, esto no debe bloquear el
    // cambio de modo (sobre todo volver a Local, que tiene que funcionar
    // siempre, sin depender de nada).
    if let StorageMode::Nube(previous) = storage_config::load(&dir) {
        let staying_on_same_business =
            mode == "nube" && nube_business_id.as_deref() == Some(previous.business_id.as_str());
        if !staying_on_same_business {
            match cognito::ensure_fresh_id_token(&dir, &state).await {
                Ok(id_token) => {
                    if let Err(e) = VendorClient::new(id_token)
                        .unlink_installation(&previous.business_id)
                        .await
                    {
                        eprintln!("no se pudo desvincular la instalación del negocio anterior de Nube: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("no se pudo obtener sesión para desvincular el negocio anterior de Nube: {e}");
                }
            }
        }
    }

    match mode.as_str() {
        "local" => {
            // "Migración de vuelta": espejo del camino "nube" (todo ANTES de
            // persistir nada) -- acá es además la única forma
            // posible de hacerlo, porque la lectura necesita que `state.db`
            // siga siendo el `AppDb::Postgres` actual (se pierde en cuanto se
            // guarda `storage.json` en modo local, ya que el próximo
            // `with_sqlite`/reinicio abre la base local en su lugar).
            let should_migrate = migrate_data.unwrap_or(false) && matches!(&state.db, AppDb::Postgres(_));
            let migration_summary = if should_migrate {
                let AppDb::Postgres(pg_pool) = &state.db else {
                    unreachable!("should_migrate ya confirmó AppDb::Postgres arriba");
                };
                let data_dir = data_dir(&app)?;
                let bucket = match bucket_config::load_bucket(&dir) {
                    Some(cfg) => {
                        let secret = bucket_config::get_bucket_secret(&cfg).map_err(|e| {
                            AppError::Internal(format!("no se pudo leer la credencial del bucket: {e}"))
                        })?;
                        let client = BucketClient::new(&cfg, &secret)?;
                        Some((cfg, client))
                    }
                    None => None,
                };
                let mut on_progress = progress_emitter(&app);
                let summary = migration::migrate_postgres_to_local(
                    &mut on_progress,
                    pg_pool,
                    &data_dir,
                    bucket.as_ref().map(|(cfg, client)| (cfg, client)),
                    replace_data.unwrap_or(false),
                )
                .await
                .map_err(|e| AppError::validation(format!("No se pudo migrar los datos: {e}")))?;
                Some(summary)
            } else {
                None
            };

            storage_config::save(&dir, &StorageMode::Local)
                .map_err(|e| AppError::Internal(format!("no se pudo guardar la configuración: {e}")))?;
            Ok(migration_summary)
        }
        "nube" => {
            let business_id =
                nube_business_id.ok_or_else(|| AppError::validation("Falta el negocio a conectar"))?;
            let business_name = nube_business_name
                .ok_or_else(|| AppError::validation("Falta el nombre del negocio"))?;
            // En Nube el bucket es opcional: el negocio se puede conectar sin
            // uno, simplemente no se podrán subir fotos del
            // catálogo hasta que se configure (ver
            // `usePhotoUploadAvailability` en el frontend y
            // `util::photos::upload_to_bucket`, que ya maneja la ausencia de
            // bucket con un error de validación en vez de romper algo).
            let bucket = match (bucket_config, bucket_secret) {
                (Some(bucket_config), Some(bucket_secret)) => {
                    let bucket_client = BucketClient::new(&bucket_config, &bucket_secret)?;
                    bucket_client
                        .test_connection()
                        .await
                        .map_err(|e| AppError::validation(format!("No se pudo validar el bucket: {e}")))?;
                    Some((bucket_config, bucket_secret, bucket_client))
                }
                (None, None) => None,
                _ => {
                    return Err(AppError::validation(
                        "Completa todos los datos del bucket o déjalos todos vacíos",
                    ))
                }
            };

            let id_token = cognito::ensure_fresh_id_token(&dir, &state).await?;
            let installation_id = vendor::get_or_create_installation_id(&dir)
                .map_err(|e| AppError::Internal(format!("no se pudo preparar el id de instalación: {e}")))?;

            // Se vuelve a pedir la conexión acá (en vez de reusar la que ya
            // trajo `vendor_connect_business` para la vista previa en el
            // frontend) -- misma semántica de heartbeat que ya define el
            // backend para la misma `installation_id`, y evita hacer viajar
            // la contraseña en texto plano por el estado del frontend entre
            // la vista previa y esta confirmación.
            let connection = match VendorClient::new(id_token)
                .get_connection(&business_id, &installation_id)
                .await?
            {
                VendorConnectOutcome::Connected(c) => c,
                VendorConnectOutcome::LinkedElsewhere => {
                    return Err(AppError::conflict(
                        "Este negocio ya está conectado desde otro equipo. Desvincúlalo primero e inténtalo de nuevo.",
                    ));
                }
            };

            let pg_config = PostgresConnectionConfig {
                host: connection.host,
                port: connection.port,
                database: connection.database,
                username: connection.user,
                sslmode: connection.ssl_mode,
            };

            let pg_pool = app_db::open_postgres(&pg_config, &connection.password)
                .await
                .map_err(|e| AppError::validation(format!("No se pudo preparar la base de datos: {e}")))?;

            let should_migrate = migrate_data.unwrap_or(false) && matches!(&state.db, AppDb::Sqlite(_));
            let migration_summary = if should_migrate {
                let data_dir = data_dir(&app)?;
                let mut on_progress = progress_emitter(&app);
                let summary = migration::migrate_local_to_postgres(
                    &mut on_progress,
                    &state.db,
                    &pg_pool,
                    &data_dir,
                    bucket.as_ref().map(|(config, _secret, client)| (config, client)),
                    replace_data.unwrap_or(false),
                )
                .await
                .map_err(|e| AppError::validation(format!("No se pudo migrar los datos locales: {e}")))?;
                // Nube ya tiene la copia: vaciar la base local evita que
                // quede una copia vieja dando vueltas (ver doc de
                // `clear_local_data`) y, sobre todo, que bloquee una futura
                // "migración de vuelta" por no estar vacía.
                state
                    .db
                    .with_sqlite(|conn| migration::clear_local_data(conn))
                    .await
                    .map_err(|e| {
                        AppError::Internal(format!(
                            "los datos se migraron bien, pero no se pudo vaciar la base local: {e}"
                        ))
                    })?;
                Some(summary)
            } else {
                None
            };

            storage_config::set_password(&pg_config, &connection.password)
                .map_err(|e| AppError::Internal(format!("no se pudo guardar la contraseña: {e}")))?;
            storage_config::save(
                &dir,
                &StorageMode::Nube(NubeConnectionConfig {
                    connection: pg_config,
                    business_id,
                    business_name,
                    installation_id,
                }),
            )
            .map_err(|e| AppError::Internal(format!("no se pudo guardar la configuración: {e}")))?;
            if let Some((bucket_config, bucket_secret, _client)) = &bucket {
                bucket_config::set_bucket_secret(bucket_config, bucket_secret).map_err(|e| {
                    AppError::Internal(format!("no se pudo guardar la credencial del bucket: {e}"))
                })?;
                bucket_config::save_bucket(&dir, bucket_config).map_err(|e| {
                    AppError::Internal(format!("no se pudo guardar la configuración del bucket: {e}"))
                })?;
            }

            Ok(migration_summary)
        }
        _ => Err(AppError::validation("Modo de almacenamiento inválido")),
    }
}

/// Descarga un respaldo JSON de los datos locales (issue #9, Fase 4). Solo
/// disponible en modo Local — el modo Nube usa la base gestionada por el
/// backend de negocio. No incluye `sessions` ni `password_hash` (ver
/// `crate::export`). `path` es elegido por el usuario en un diálogo nativo
/// de "Guardar como" en el frontend, no una ruta arbitraria del backend.
#[tauri::command]
pub async fn storage_export_local_data(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let actor = state.require_owner()?;

    match &state.db {
        AppDb::Sqlite(_) => {
            let json = state.db.with_sqlite(export::export_all).await?;
            std::fs::write(&path, json)
                .map_err(|e| AppError::Internal(format!("no se pudo escribir el archivo: {e}")))?;
            audit::record(
                &state.db,
                Some(&actor),
                NewEntry::event("data.export", None, "Exportó un respaldo completo de los datos"),
            )
            .await;
            Ok(())
        }
        AppDb::Postgres(_) => Err(AppError::validation(
            "El respaldo a JSON solo está disponible en modo local",
        )),
    }
}
