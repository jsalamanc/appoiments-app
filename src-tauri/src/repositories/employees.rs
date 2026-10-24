use rusqlite::{params, Connection};

use crate::commands::employees::EmployeeInput;
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::employee::Employee;

/// Replica la validación `^#[0-9A-Fa-f]{6}$` de Postgres (SQLite no soporta
/// regex en `CHECK`, así que vive aquí).
fn validate_color(color: &str) -> AppResult<()> {
    let bytes = color.as_bytes();
    let valid = bytes.len() == 7
        && color.is_ascii()
        && bytes[0] == b'#'
        && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(AppError::validation(
            "El color debe tener el formato #RRGGBB",
        ));
    }
    Ok(())
}

pub fn list(conn: &mut Connection, active_only: Option<bool>) -> AppResult<Vec<Employee>> {
    let active_only = active_only.unwrap_or(false);

    let mut stmt = if active_only {
        conn.prepare("SELECT * FROM employees WHERE active = 1 ORDER BY name ASC")?
    } else {
        conn.prepare("SELECT * FROM employees ORDER BY name ASC")?
    };
    let rows = stmt.query_map([], Employee::from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn create(conn: &mut Connection, input: EmployeeInput) -> AppResult<Employee> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("El nombre es obligatorio"));
    }
    let color = input.color.trim().to_string();
    validate_color(&color)?;

    let id = new_id();
    let now = now_iso();

    conn.execute(
        "INSERT INTO employees (id, name, color, active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, name, color, input.active as i64, now],
    )?;

    let employee = conn.query_row(
        "SELECT * FROM employees WHERE id = ?1",
        params![id],
        Employee::from_row,
    )?;
    Ok(employee)
}

pub fn update(conn: &mut Connection, id: String, input: EmployeeInput) -> AppResult<Employee> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("El nombre es obligatorio"));
    }
    let color = input.color.trim().to_string();
    validate_color(&color)?;

    conn.execute(
        "UPDATE employees SET name = ?1, color = ?2, active = ?3, updated_at = ?4
         WHERE id = ?5",
        params![name, color, input.active as i64, now_iso(), id],
    )?;

    let employee = conn.query_row(
        "SELECT * FROM employees WHERE id = ?1",
        params![id],
        Employee::from_row,
    )?;
    Ok(employee)
}

/// Falla con un mensaje claro si el empleado tiene citas asociadas: el
/// `ON DELETE RESTRICT` de `appointments.employee_id` dispara una violación
/// de llave foránea. SQLite no incluye el nombre de la columna en el mensaje
/// de error de una violación de FK (a diferencia de UNIQUE), así que
/// `AppError::from(rusqlite::Error)` (ver src-tauri/src/error.rs) solo puede
/// dar un mensaje genérico de conflicto; aquí lo reemplazamos por el mismo
/// mensaje que ya mostraba la UI en la era Supabase, guiando a desactivar en
/// vez de eliminar.
pub fn delete(conn: &mut Connection, id: String) -> AppResult<()> {
    conn.execute("DELETE FROM employees WHERE id = ?1", params![id])
        .map_err(|e| match AppError::from(e) {
            AppError::Conflict(_) => AppError::conflict(
                "No se puede eliminar: el empleado tiene citas asociadas. Desactívalo en su lugar.",
            ),
            other => other,
        })?;
    Ok(())
}

/// Implementación PostgreSQL (issue #9, Fase 2) — mismas reglas que arriba,
/// adaptadas al driver async `sqlx`. `active` decodifica directo como
/// `BOOLEAN` nativo (sin el `!= 0`/`as i64` de SQLite), y `Employee::from_pg_row`
/// (ver `models/employee.rs`) resuelve el mapeo de filas, incluida la
/// conversión de `TIMESTAMPTZ` a `String` RFC3339.
pub mod postgres {
    use sqlx::PgPool;

    use super::validate_color;
    use crate::commands::employees::EmployeeInput;
    use crate::db::new_id;
    use crate::error::{AppError, AppResult};
    use crate::models::employee::Employee;

    pub async fn list(pool: &PgPool, active_only: Option<bool>) -> AppResult<Vec<Employee>> {
        let active_only = active_only.unwrap_or(false);

        let rows = if active_only {
            sqlx::query("SELECT * FROM employees WHERE active = true ORDER BY name ASC")
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query("SELECT * FROM employees ORDER BY name ASC")
                .fetch_all(pool)
                .await?
        };

        rows.iter().map(Employee::from_pg_row).collect::<AppResult<Vec<_>>>()
    }

    pub async fn create(pool: &PgPool, input: EmployeeInput) -> AppResult<Employee> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::validation("El nombre es obligatorio"));
        }
        let color = input.color.trim().to_string();
        validate_color(&color)?;

        let row = sqlx::query(
            "INSERT INTO employees (id, name, color, active)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(new_id())
        .bind(&name)
        .bind(&color)
        .bind(input.active)
        .fetch_one(pool)
        .await?;

        Employee::from_pg_row(&row)
    }

    pub async fn update(pool: &PgPool, id: String, input: EmployeeInput) -> AppResult<Employee> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::validation("El nombre es obligatorio"));
        }
        let color = input.color.trim().to_string();
        validate_color(&color)?;

        let row = sqlx::query(
            "UPDATE employees SET name = $1, color = $2, active = $3, updated_at = now()
             WHERE id = $4
             RETURNING *",
        )
        .bind(&name)
        .bind(&color)
        .bind(input.active)
        .bind(&id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;

        Employee::from_pg_row(&row)
    }

    /// Mismo mensaje amigable que la versión SQLite cuando la violación de
    /// llave foránea de `appointments.employee_id` (`ON DELETE RESTRICT`)
    /// dispara un error de Postgres (código `23503`), que
    /// `AppError::from(sqlx::Error)` ya traduce a `AppError::Conflict` con un
    /// mensaje genérico (ver `error.rs`).
    pub async fn delete(pool: &PgPool, id: String) -> AppResult<()> {
        sqlx::query("DELETE FROM employees WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| match AppError::from(e) {
                AppError::Conflict(_) => AppError::conflict(
                    "No se puede eliminar: el empleado tiene citas asociadas. Desactívalo en su lugar.",
                ),
                other => other,
            })?;
        Ok(())
    }
}
