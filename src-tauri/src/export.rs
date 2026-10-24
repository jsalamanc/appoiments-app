//! Respaldo de los datos locales a un único archivo JSON (issue #9, Fase 4),
//! para que el usuario pueda descargarlo antes de cambiar de modo de
//! almacenamiento. Solo tiene sentido en modo Local (SQLite) — no se ofrece
//! para el modo Postgres, donde el usuario ya administra su propio servidor
//! y sus propios respaldos.
//!
//! Deliberadamente **no** incluye la tabla `sessions` (tokens de sesión
//! vigentes) ni la columna `password_hash` de `users`: un respaldo de datos
//! del negocio no debe poder usarse para iniciar sesión como otra persona.

use rusqlite::types::ValueRef;
use rusqlite::{Connection, Row};
use serde_json::{Map, Value};

use crate::db::now_iso;
use crate::error::AppResult;

fn value_ref_to_json(value: ValueRef) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => Value::Number(n.into()),
        ValueRef::Real(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
        // El esquema actual no guarda BLOBs; se deja como null por si acaso.
        ValueRef::Blob(_) => Value::Null,
    }
}

fn row_to_json(row: &Row, columns: &[String]) -> rusqlite::Result<Map<String, Value>> {
    let mut map = Map::with_capacity(columns.len());
    for (i, name) in columns.iter().enumerate() {
        map.insert(name.clone(), value_ref_to_json(row.get_ref(i)?));
    }
    Ok(map)
}

/// Exporta todas las columnas de `table`. Solo se llama con literales fijos
/// definidos en este archivo, nunca con datos del usuario.
fn export_table(conn: &Connection, table: &str) -> AppResult<Vec<Value>> {
    export_columns(conn, table, "*")
}

fn export_columns(conn: &Connection, table: &str, columns_sql: &str) -> AppResult<Vec<Value>> {
    let mut stmt = conn.prepare(&format!("SELECT {columns_sql} FROM {table}"))?;
    let columns: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let rows = stmt
        .query_map([], |row| row_to_json(row, &columns))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows.into_iter().map(Value::Object).collect())
}

/// Arma el respaldo completo como un string JSON "pretty". Pensado para
/// correr dentro de `AppDb::with_sqlite` (ver `commands::storage::
/// storage_export_local_data`), no se expone como comando directamente.
pub fn export_all(conn: &mut Connection) -> AppResult<String> {
    let mut root = Map::new();
    root.insert("exportedAt".into(), Value::String(now_iso()));
    root.insert(
        "source".into(),
        Value::String("aura-studio-local-sqlite".into()),
    );

    root.insert(
        "businessProfile".into(),
        Value::Array(export_table(conn, "business_profile")?),
    );
    // Sin password_hash: un respaldo de datos no debe servir para
    // suplantar usuarios.
    root.insert(
        "users".into(),
        Value::Array(export_columns(
            conn,
            "users",
            "id, name, username, role, active, created_at, updated_at",
        )?),
    );
    root.insert("customers".into(), Value::Array(export_table(conn, "customers")?));
    root.insert("products".into(), Value::Array(export_table(conn, "products")?));
    root.insert("services".into(), Value::Array(export_table(conn, "services")?));
    root.insert("packages".into(), Value::Array(export_table(conn, "packages")?));
    root.insert(
        "packageItems".into(),
        Value::Array(export_table(conn, "package_items")?),
    );
    root.insert("employees".into(), Value::Array(export_table(conn, "employees")?));
    root.insert(
        "appointments".into(),
        Value::Array(export_table(conn, "appointments")?),
    );
    root.insert("sales".into(), Value::Array(export_table(conn, "sales")?));
    root.insert("saleItems".into(), Value::Array(export_table(conn, "sale_items")?));
    root.insert(
        "inventoryMovements".into(),
        Value::Array(export_table(conn, "inventory_movements")?),
    );

    serde_json::to_string_pretty(&Value::Object(root))
        .map_err(|e| crate::error::AppError::Internal(format!("no se pudo generar el respaldo: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations().to_latest(&mut conn).unwrap();
        conn
    }

    #[test]
    fn export_excludes_password_hash_and_sessions() {
        let mut conn = open_test_db();
        conn.execute(
            "INSERT INTO users (id, name, username, password_hash, role, active, created_at, updated_at)
             VALUES ('u1', 'Owner', 'owner', 'super-secret-hash', 'owner', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sessions (token, user_id, created_at, expires_at)
             VALUES ('tok-123', 'u1', '2026-01-01T00:00:00Z', '2026-02-01T00:00:00Z')",
            [],
        )
        .unwrap();

        let json = export_all(&mut conn).unwrap();
        assert!(
            !json.contains("super-secret-hash"),
            "el respaldo no debe incluir password_hash"
        );
        assert!(
            !json.contains("tok-123"),
            "el respaldo no debe incluir tokens de sesión"
        );
        assert!(json.contains("\"username\": \"owner\""));

        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("sessions").is_none(), "no debe existir la clave \"sessions\"");
        assert_eq!(parsed["users"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn export_handles_empty_database() {
        let mut conn = open_test_db();
        let json = export_all(&mut conn).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["customers"].as_array().unwrap().len(), 0);
    }
}
