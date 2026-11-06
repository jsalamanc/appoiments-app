use std::env;

/// Estas tres no son secretos (son identificadores públicos: aparecen en el
/// `issuer`/`aud` de cualquier JWT emitido, o son simplemente la URL del API
/// Gateway) -- son variables de entorno igual porque dev y producción son
/// ambientes de AWS separados (otro User Pool de Cognito, otro API Gateway),
/// y el valor tiene que poder cambiar según dónde se compile, sin tocar
/// código fuente.
///
/// Se resuelven en tiempo de COMPILACIÓN, no en tiempo de ejecución: un
/// usuario final instalando la app no tiene (ni debería tener) estas
/// variables seteadas en su máquina, así que no alcanza con leerlas vía
/// `std::env::var` desde el binario ya compilado -- hay que "hornearlas"
/// adentro con `cargo:rustc-env`, que es lo que lee `env!(...)` en el código
/// (ver cognito/mod.rs y vendor/mod.rs).
const REQUIRED_ENV_VARS: &[&str] = &[
    "COGNITO_USER_POOL_ID",
    "COGNITO_APP_CLIENT_ID",
    "VENDOR_API_BASE_URL",
];

fn main() {
    // Conveniencia solo para desarrollo local: `cargo build`/`tauri dev`
    // corridos a mano no traen estas variables en el shell, así que se
    // completan desde `.env.local` (gitignored, mismo patrón que
    // aura-studio-landing) si existe. En CI (release.yml) no existe ese
    // archivo -- ahí las variables ya vienen puestas directo en el entorno
    // del step, así que esto no hace nada (ni falla si el archivo falta).
    let _ = dotenvy::from_filename(".env.local");
    println!("cargo:rerun-if-changed=.env.local");

    for key in REQUIRED_ENV_VARS {
        println!("cargo:rerun-if-env-changed={key}");
        match env::var(key) {
            Ok(value) => println!("cargo:rustc-env={key}={value}"),
            Err(_) => panic!(
                "Falta la variable de entorno {key} para compilar.\n\
                 En local: copiá src-tauri/.env.local.example a src-tauri/.env.local y completalo con los valores de dev.\n\
                 En CI: tiene que venir seteada en el step que compila (ver .github/workflows/release.yml)."
            ),
        }
    }

    tauri_build::build();
}
