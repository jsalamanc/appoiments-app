//! Horario de atención del negocio y disponibilidad por empleado (issue #2,
//! Fase 3). Ver `commands::business_hours` para la convención de `weekday`.

use rusqlite::{params, Connection, OptionalExtension};

use crate::commands::business_hours::{BusinessHoursInput, EmployeeAvailabilityInput};
use crate::db::new_id;
use crate::error::{AppError, AppResult};
use crate::models::business_hours::{BusinessHours, EmployeeAvailability};

/// Convierte "HH:MM" a minutos desde medianoche. No se intenta validar nada
/// más estricto (ej. rango 0-23 en la hora) porque ya viene de un `<input
/// type="time">` del frontend — si llegara corrupto, que falle acá con un
/// error claro en vez de guardar basura.
fn parse_time_of_day(value: &str) -> AppResult<i64> {
    let (h, m) = value
        .split_once(':')
        .ok_or_else(|| AppError::validation(format!("Hora inválida: \"{value}\"")))?;
    let h: i64 = h
        .parse()
        .map_err(|_| AppError::validation(format!("Hora inválida: \"{value}\"")))?;
    let m: i64 = m
        .parse()
        .map_err(|_| AppError::validation(format!("Hora inválida: \"{value}\"")))?;
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return Err(AppError::validation(format!("Hora inválida: \"{value}\"")));
    }
    Ok(h * 60 + m)
}

fn validate_window(is_open: bool, opens_at: &Option<String>, closes_at: &Option<String>) -> AppResult<()> {
    if !is_open {
        return Ok(());
    }
    let (open, close) = match (opens_at, closes_at) {
        (Some(o), Some(c)) => (parse_time_of_day(o)?, parse_time_of_day(c)?),
        _ => {
            return Err(AppError::validation(
                "Un día abierto necesita hora de apertura y cierre",
            ))
        }
    };
    if open >= close {
        return Err(AppError::validation(
            "La hora de apertura debe ser antes que la de cierre",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// business_hours
// ---------------------------------------------------------------------------

pub fn list_business_hours(conn: &mut Connection) -> AppResult<Vec<BusinessHours>> {
    let mut stmt = conn.prepare("SELECT * FROM business_hours ORDER BY weekday ASC")?;
    let rows = stmt
        .query_map([], BusinessHours::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Las 7 filas ya existen desde la migración (sembradas con un horario por
/// defecto) — esto siempre actualiza filas existentes, nunca inserta ni
/// borra, así que no hace falta preocuparse por "faltan días": los que no
/// vengan en `hours` simplemente quedan como estaban.
pub fn update_business_hours(
    conn: &mut Connection,
    hours: Vec<BusinessHoursInput>,
) -> AppResult<Vec<BusinessHours>> {
    for entry in &hours {
        if !(0..7).contains(&entry.weekday) {
            return Err(AppError::validation("Día de la semana inválido"));
        }
        validate_window(entry.is_open, &entry.opens_at, &entry.closes_at)?;
    }

    let tx = conn.transaction()?;
    for entry in &hours {
        let (opens_at, closes_at) = if entry.is_open {
            (entry.opens_at.clone(), entry.closes_at.clone())
        } else {
            (None, None)
        };
        tx.execute(
            "UPDATE business_hours SET is_open = ?1, opens_at = ?2, closes_at = ?3 WHERE weekday = ?4",
            params![entry.is_open as i64, opens_at, closes_at, entry.weekday],
        )?;
    }
    tx.commit()?;

    list_business_hours(conn)
}

// ---------------------------------------------------------------------------
// employee_availability
// ---------------------------------------------------------------------------

pub fn list_employee_availability(
    conn: &mut Connection,
    employee_id: String,
) -> AppResult<Vec<EmployeeAvailability>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM employee_availability WHERE employee_id = ?1 ORDER BY weekday ASC",
    )?;
    let rows = stmt
        .query_map(params![employee_id], EmployeeAvailability::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Reemplaza todas las excepciones del empleado: borra lo que había y
/// reinserta `overrides` — más simple y menos propenso a errores que tratar
/// de calcular un diff, y el frontend siempre manda el estado completo de
/// los días que sí tienen excepción (ver `commands::business_hours`).
pub fn update_employee_availability(
    conn: &mut Connection,
    employee_id: String,
    overrides: Vec<EmployeeAvailabilityInput>,
) -> AppResult<Vec<EmployeeAvailability>> {
    for entry in &overrides {
        if !(0..7).contains(&entry.weekday) {
            return Err(AppError::validation("Día de la semana inválido"));
        }
        if entry.is_available {
            if let (Some(s), Some(e)) = (&entry.starts_at, &entry.ends_at) {
                if parse_time_of_day(s)? >= parse_time_of_day(e)? {
                    return Err(AppError::validation(
                        "La hora de inicio debe ser antes que la de fin",
                    ));
                }
            }
        }
    }

    let tx = conn.transaction()?;

    let employee_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM employees WHERE id = ?1)",
        params![employee_id],
        |r| r.get(0),
    )?;
    if !employee_exists {
        return Err(AppError::validation("El empleado no existe"));
    }

    tx.execute(
        "DELETE FROM employee_availability WHERE employee_id = ?1",
        params![employee_id],
    )?;
    for entry in &overrides {
        let (starts_at, ends_at) = if entry.is_available {
            (entry.starts_at.clone(), entry.ends_at.clone())
        } else {
            (None, None)
        };
        tx.execute(
            "INSERT INTO employee_availability (id, employee_id, weekday, is_available, starts_at, ends_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![new_id(), employee_id, entry.weekday, entry.is_available as i64, starts_at, ends_at],
        )?;
    }
    tx.commit()?;

    list_employee_availability(conn, employee_id)
}

// ---------------------------------------------------------------------------
// Validación usada por repositories::appointments
// ---------------------------------------------------------------------------

/// Ventana disponible (minutos desde medianoche) del horario de negocio para
/// `weekday`, o `None` si el negocio está cerrado ese día.
fn business_window(conn: &Connection, weekday: i64) -> AppResult<Option<(i64, i64)>> {
    let row: Option<(i64, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT is_open, opens_at, closes_at FROM business_hours WHERE weekday = ?1",
            params![weekday],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    match row {
        Some((is_open, Some(opens_at), Some(closes_at))) if is_open != 0 => {
            Ok(Some((parse_time_of_day(&opens_at)?, parse_time_of_day(&closes_at)?)))
        }
        _ => Ok(None),
    }
}

/// Exige que `[start_minutes, end_minutes)` quepa dentro de la disponibilidad
/// real del empleado el día `weekday`: su excepción propia si tiene una
/// (`employee_availability`), o si no, el horario de negocio de ese día.
///
/// Se llama desde `repositories::appointments::create`/`update`, con los
/// mismos `weekday`/minutos ya resueltos en hora LOCAL por el frontend (ver
/// el comentario en `commands::appointments::AppointmentInput` sobre por qué
/// esto no se calcula en Rust a partir del `starts_at` en UTC).
pub fn assert_appointment_within_schedule(
    conn: &Connection,
    employee_id: &str,
    weekday: i64,
    start_minutes: i64,
    end_minutes: i64,
) -> AppResult<()> {
    let override_row: Option<(i64, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT is_available, starts_at, ends_at FROM employee_availability
             WHERE employee_id = ?1 AND weekday = ?2",
            params![employee_id, weekday],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    let window = match override_row {
        Some((0, _, _)) => None,
        Some((_, Some(starts_at), Some(ends_at))) => {
            Some((parse_time_of_day(&starts_at)?, parse_time_of_day(&ends_at)?))
        }
        // Disponible sin horas propias (o sin excepción registrada): sigue
        // el horario de negocio de ese día.
        _ => business_window(conn, weekday)?,
    };

    let Some((open, close)) = window else {
        return Err(AppError::validation(
            "El empleado no atiende ese día de la semana",
        ));
    };
    if start_minutes < open || end_minutes > close {
        return Err(AppError::validation(
            "Ese horario está fuera de la disponibilidad del empleado",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations().to_latest(&mut conn).unwrap();
        conn
    }

    fn seed_employee(conn: &Connection) {
        conn.execute(
            "INSERT INTO employees (id, name, color, active) VALUES ('emp-1', 'Ana', '#fff', 1)",
            [],
        )
        .unwrap();
    }

    fn hours_input(weekday: i64, is_open: bool, opens_at: Option<&str>, closes_at: Option<&str>) -> BusinessHoursInput {
        BusinessHoursInput {
            weekday,
            is_open,
            opens_at: opens_at.map(str::to_string),
            closes_at: closes_at.map(str::to_string),
        }
    }

    #[test]
    fn parse_time_of_day_parses_valid_values() {
        assert_eq!(parse_time_of_day("00:00").unwrap(), 0);
        assert_eq!(parse_time_of_day("09:30").unwrap(), 9 * 60 + 30);
        assert_eq!(parse_time_of_day("23:59").unwrap(), 23 * 60 + 59);
    }

    #[test]
    fn parse_time_of_day_rejects_malformed_values() {
        assert!(parse_time_of_day("25:00").is_err(), "hora fuera de rango");
        assert!(parse_time_of_day("10:60").is_err(), "minutos fuera de rango");
        assert!(parse_time_of_day("10").is_err(), "sin separador");
        assert!(parse_time_of_day("ab:cd").is_err(), "no numérico");
    }

    #[test]
    fn update_business_hours_rejects_invalid_weekday() {
        let mut conn = open_test_db();

        let result = update_business_hours(&mut conn, vec![hours_input(7, true, Some("09:00"), Some("17:00"))]);

        assert!(result.is_err(), "weekday 7 está fuera del rango 0..7");
    }

    #[test]
    fn update_business_hours_rejects_open_without_window() {
        let mut conn = open_test_db();

        let result = update_business_hours(&mut conn, vec![hours_input(1, true, None, None)]);

        assert!(result.is_err(), "un día abierto necesita apertura y cierre");
    }

    #[test]
    fn update_business_hours_rejects_close_before_open() {
        let mut conn = open_test_db();

        let result = update_business_hours(&mut conn, vec![hours_input(1, true, Some("18:00"), Some("09:00"))]);

        assert!(result.is_err(), "la apertura debe ser antes que el cierre");
    }

    #[test]
    fn update_business_hours_clears_window_when_closed() {
        let mut conn = open_test_db();

        let updated = update_business_hours(&mut conn, vec![hours_input(1, false, None, None)]).unwrap();

        let monday = updated.iter().find(|h| h.weekday == 1).unwrap();
        assert!(!monday.is_open);
        assert!(monday.opens_at.is_none());
        assert!(monday.closes_at.is_none());
    }

    #[test]
    fn update_employee_availability_rejects_unknown_employee() {
        let mut conn = open_test_db();

        let result = update_employee_availability(
            &mut conn,
            "missing-emp".to_string(),
            vec![EmployeeAvailabilityInput {
                weekday: 0,
                is_available: true,
                starts_at: Some("10:00".to_string()),
                ends_at: Some("14:00".to_string()),
            }],
        );

        assert!(result.is_err(), "no debe poder crear excepciones para un empleado inexistente");
    }

    #[test]
    fn update_employee_availability_rejects_start_after_end() {
        let mut conn = open_test_db();
        seed_employee(&conn);

        let result = update_employee_availability(
            &mut conn,
            "emp-1".to_string(),
            vec![EmployeeAvailabilityInput {
                weekday: 0,
                is_available: true,
                starts_at: Some("14:00".to_string()),
                ends_at: Some("10:00".to_string()),
            }],
        );

        assert!(result.is_err(), "el inicio debe ser antes que el fin");
    }

    /// `update_employee_availability` reemplaza por completo el conjunto de
    /// excepciones del empleado en cada llamada — una segunda llamada con una
    /// lista distinta no debe dejar restos de la primera.
    #[test]
    fn update_employee_availability_replaces_previous_overrides() {
        let mut conn = open_test_db();
        seed_employee(&conn);

        update_employee_availability(
            &mut conn,
            "emp-1".to_string(),
            vec![EmployeeAvailabilityInput {
                weekday: 0,
                is_available: false,
                starts_at: None,
                ends_at: None,
            }],
        )
        .unwrap();

        let second = update_employee_availability(
            &mut conn,
            "emp-1".to_string(),
            vec![EmployeeAvailabilityInput {
                weekday: 2,
                is_available: true,
                starts_at: Some("10:00".to_string()),
                ends_at: Some("12:00".to_string()),
            }],
        )
        .unwrap();

        assert_eq!(second.len(), 1, "la excepción del domingo (weekday 0) debe haberse reemplazado");
        assert_eq!(second[0].weekday, 2);
    }
}

pub mod postgres {
    use sqlx::{PgPool, Row};

    use crate::commands::business_hours::{BusinessHoursInput, EmployeeAvailabilityInput};
    use crate::db::new_id;
    use crate::error::{AppError, AppResult};
    use crate::models::business_hours::{BusinessHours, EmployeeAvailability};

    use super::{parse_time_of_day, validate_window};

    pub async fn list_business_hours(pool: &PgPool) -> AppResult<Vec<BusinessHours>> {
        let rows = sqlx::query("SELECT * FROM business_hours ORDER BY weekday ASC")
            .fetch_all(pool)
            .await?;
        rows.iter().map(BusinessHours::from_pg_row).collect()
    }

    pub async fn update_business_hours(
        pool: &PgPool,
        hours: Vec<BusinessHoursInput>,
    ) -> AppResult<Vec<BusinessHours>> {
        for entry in &hours {
            if !(0..7).contains(&entry.weekday) {
                return Err(AppError::validation("Día de la semana inválido"));
            }
            validate_window(entry.is_open, &entry.opens_at, &entry.closes_at)?;
        }

        let mut tx = pool.begin().await?;
        for entry in &hours {
            let (opens_at, closes_at) = if entry.is_open {
                (entry.opens_at.clone(), entry.closes_at.clone())
            } else {
                (None, None)
            };
            sqlx::query(
                "UPDATE business_hours SET is_open = $1, opens_at = $2, closes_at = $3 WHERE weekday = $4",
            )
            .bind(entry.is_open)
            .bind(opens_at)
            .bind(closes_at)
            .bind(entry.weekday as i16)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        list_business_hours(pool).await
    }

    pub async fn list_employee_availability(
        pool: &PgPool,
        employee_id: String,
    ) -> AppResult<Vec<EmployeeAvailability>> {
        let rows = sqlx::query(
            "SELECT * FROM employee_availability WHERE employee_id = $1 ORDER BY weekday ASC",
        )
        .bind(employee_id)
        .fetch_all(pool)
        .await?;
        rows.iter().map(EmployeeAvailability::from_pg_row).collect()
    }

    pub async fn update_employee_availability(
        pool: &PgPool,
        employee_id: String,
        overrides: Vec<EmployeeAvailabilityInput>,
    ) -> AppResult<Vec<EmployeeAvailability>> {
        for entry in &overrides {
            if !(0..7).contains(&entry.weekday) {
                return Err(AppError::validation("Día de la semana inválido"));
            }
            if entry.is_available {
                if let (Some(s), Some(e)) = (&entry.starts_at, &entry.ends_at) {
                    if parse_time_of_day(s)? >= parse_time_of_day(e)? {
                        return Err(AppError::validation(
                            "La hora de inicio debe ser antes que la de fin",
                        ));
                    }
                }
            }
        }

        let mut tx = pool.begin().await?;

        let employee_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM employees WHERE id = $1)")
                .bind(&employee_id)
                .fetch_one(&mut *tx)
                .await?;
        if !employee_exists {
            return Err(AppError::validation("El empleado no existe"));
        }

        sqlx::query("DELETE FROM employee_availability WHERE employee_id = $1")
            .bind(&employee_id)
            .execute(&mut *tx)
            .await?;
        for entry in &overrides {
            let (starts_at, ends_at) = if entry.is_available {
                (entry.starts_at.clone(), entry.ends_at.clone())
            } else {
                (None, None)
            };
            sqlx::query(
                "INSERT INTO employee_availability (id, employee_id, weekday, is_available, starts_at, ends_at)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(new_id())
            .bind(&employee_id)
            .bind(entry.weekday as i16)
            .bind(entry.is_available)
            .bind(starts_at)
            .bind(ends_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        list_employee_availability(pool, employee_id).await
    }

    async fn business_window(
        conn: &mut sqlx::PgConnection,
        weekday: i64,
    ) -> AppResult<Option<(i64, i64)>> {
        let row = sqlx::query(
            "SELECT is_open, opens_at, closes_at FROM business_hours WHERE weekday = $1",
        )
        .bind(weekday as i16)
        .fetch_optional(&mut *conn)
        .await?;
        let Some(row) = row else { return Ok(None) };
        let is_open: bool = row.try_get("is_open")?;
        let opens_at: Option<String> = row.try_get("opens_at")?;
        let closes_at: Option<String> = row.try_get("closes_at")?;
        match (is_open, opens_at, closes_at) {
            (true, Some(o), Some(c)) => Ok(Some((parse_time_of_day(&o)?, parse_time_of_day(&c)?))),
            _ => Ok(None),
        }
    }

    /// Ver `super::assert_appointment_within_schedule` — misma lógica,
    /// dialecto Postgres. Toma `&mut PgConnection` concreto (en vez de ser
    /// genérico sobre `PgExecutor`) porque necesita hacer más de una consulta
    /// secuencial sobre la misma conexión dentro de la transacción de
    /// `repositories::appointments::postgres::create`/`update` — un executor
    /// genérico por valor (como el resto de los asserts de ese archivo, que
    /// solo hacen una consulta cada uno) no se puede reusar dos veces.
    pub async fn assert_appointment_within_schedule(
        conn: &mut sqlx::PgConnection,
        employee_id: &str,
        weekday: i64,
        start_minutes: i64,
        end_minutes: i64,
    ) -> AppResult<()> {
        let override_row = sqlx::query(
            "SELECT is_available, starts_at, ends_at FROM employee_availability
             WHERE employee_id = $1 AND weekday = $2",
        )
        .bind(employee_id)
        .bind(weekday as i16)
        .fetch_optional(&mut *conn)
        .await?;

        let window = match override_row {
            Some(row) => {
                let is_available: bool = row.try_get("is_available")?;
                if !is_available {
                    None
                } else {
                    let starts_at: Option<String> = row.try_get("starts_at")?;
                    let ends_at: Option<String> = row.try_get("ends_at")?;
                    match (starts_at, ends_at) {
                        (Some(s), Some(e)) => {
                            Some((parse_time_of_day(&s)?, parse_time_of_day(&e)?))
                        }
                        _ => business_window(conn, weekday).await?,
                    }
                }
            }
            None => business_window(conn, weekday).await?,
        };

        let Some((open, close)) = window else {
            return Err(AppError::validation(
                "El empleado no atiende ese día de la semana",
            ));
        };
        if start_minutes < open || end_minutes > close {
            return Err(AppError::validation(
                "Ese horario está fuera de la disponibilidad del empleado",
            ));
        }
        Ok(())
    }
}
