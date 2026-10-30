//! Comisión por empleado sobre las citas completadas (issue #22, #2 Fase 4).
//! Ver `commands::commissions` para la convención de la API.

use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::commands::commissions::CommissionRateInput;
use crate::db::new_id;
use crate::error::{AppError, AppResult};
use crate::models::commission::EmployeeCommissionRate;

/// Clave de `rates_by_scope`: (employee_id, service_id). `service_id = None`
/// es la tasa general del empleado.
pub type RateKey = (String, Option<String>);
pub type RateValue = (String, f64);

fn validate_rate(rate_type: &str, rate_value: f64) -> AppResult<()> {
    if rate_type != "percentage" && rate_type != "fixed" {
        return Err(AppError::validation(
            "El tipo de comisión debe ser \"percentage\" o \"fixed\"",
        ));
    }
    if rate_value < 0.0 {
        return Err(AppError::validation("La comisión no puede ser negativa"));
    }
    if rate_type == "percentage" && rate_value > 100.0 {
        return Err(AppError::validation("Un porcentaje no puede ser mayor a 100"));
    }
    Ok(())
}

/// Ninguna entrada repite `service_id` (incluyendo como máximo una tasa
/// general con `service_id: None`).
fn validate_no_duplicate_scopes(rates: &[CommissionRateInput]) -> AppResult<()> {
    let mut seen: Vec<&Option<String>> = Vec::with_capacity(rates.len());
    for entry in rates {
        if seen.contains(&&entry.service_id) {
            return Err(AppError::validation(
                "No puede haber dos tasas de comisión para el mismo servicio",
            ));
        }
        seen.push(&entry.service_id);
    }
    Ok(())
}

pub fn list_employee_commission_rates(
    conn: &mut Connection,
    employee_id: String,
) -> AppResult<Vec<EmployeeCommissionRate>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM employee_commission_rates WHERE employee_id = ?1
         ORDER BY service_id IS NULL DESC, service_id ASC",
    )?;
    let rows = stmt
        .query_map(params![employee_id], EmployeeCommissionRate::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Reemplaza todas las tasas del empleado — borra lo que había y reinserta
/// `rates`, igual convención que `update_employee_availability`.
pub fn update_employee_commission_rates(
    conn: &mut Connection,
    employee_id: String,
    rates: Vec<CommissionRateInput>,
) -> AppResult<Vec<EmployeeCommissionRate>> {
    for entry in &rates {
        validate_rate(&entry.rate_type, entry.rate_value)?;
    }
    validate_no_duplicate_scopes(&rates)?;

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
        "DELETE FROM employee_commission_rates WHERE employee_id = ?1",
        params![employee_id],
    )?;
    for entry in &rates {
        tx.execute(
            "INSERT INTO employee_commission_rates (id, employee_id, service_id, rate_type, rate_value)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![new_id(), employee_id, entry.service_id, entry.rate_type, entry.rate_value],
        )?;
    }
    tx.commit()?;

    list_employee_commission_rates(conn, employee_id)
}

/// Carga todas las tasas configuradas (tabla pequeña, una fila por empleado
/// y, opcionalmente, por excepción de servicio) en un mapa para resolución
/// en memoria — usado por `repositories::reports::employee_commissions`
/// para no hacer una consulta por cita.
pub fn load_rate_map(conn: &Connection) -> AppResult<HashMap<RateKey, RateValue>> {
    let mut stmt =
        conn.prepare("SELECT employee_id, service_id, rate_type, rate_value FROM employee_commission_rates")?;
    let rows = stmt
        .query_map([], |row| {
            let employee_id: String = row.get(0)?;
            let service_id: Option<String> = row.get(1)?;
            let rate_type: String = row.get(2)?;
            let rate_value: f64 = row.get(3)?;
            Ok(((employee_id, service_id), (rate_type, rate_value)))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;
    Ok(rows)
}

/// Tasa aplicable a una cita de `employee_id` con `service_id`: la excepción
/// de ese servicio si existe, si no la tasa general del empleado, si no
/// ninguna (sin comisión configurada).
pub fn resolve_rate<'a>(
    rate_map: &'a HashMap<RateKey, RateValue>,
    employee_id: &str,
    service_id: Option<&str>,
) -> Option<&'a RateValue> {
    if let Some(service_id) = service_id {
        if let Some(rate) = rate_map.get(&(employee_id.to_string(), Some(service_id.to_string())))
        {
            return Some(rate);
        }
    }
    rate_map.get(&(employee_id.to_string(), None))
}

/// Monto de comisión de una cita cuya venta vinculada facturó `sale_total`
/// (pesos enteros — ver issue #2, Fase 6). `"percentage"` se calcula sobre
/// `sale_total` y puede dar un resultado fraccionario (ej. 12.5% de
/// $1.001.000); `"fixed"` es un monto plano por cita, independiente del
/// precio cobrado. El resultado es solo para reporte, nunca se guarda en la
/// base de datos, así que queda en `f64` en vez de redondearlo a un entero.
pub fn commission_amount(rate: &RateValue, sale_total: i64) -> f64 {
    let (rate_type, rate_value) = rate;
    if rate_type == "percentage" {
        sale_total as f64 * rate_value / 100.0
    } else {
        *rate_value
    }
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

    fn seed_service(conn: &Connection) {
        conn.execute(
            "INSERT INTO services (id, name, price, duration_minutes, status)
             VALUES ('svc-1', 'Corte', 20.0, 60, 'published')",
            [],
        )
        .unwrap();
    }

    fn rate_input(service_id: Option<&str>, rate_type: &str, rate_value: f64) -> CommissionRateInput {
        CommissionRateInput {
            service_id: service_id.map(str::to_string),
            rate_type: rate_type.to_string(),
            rate_value,
        }
    }

    #[test]
    fn update_rates_rejects_unknown_rate_type() {
        let mut conn = open_test_db();
        seed_employee(&conn);

        let result = update_employee_commission_rates(
            &mut conn,
            "emp-1".to_string(),
            vec![rate_input(None, "percent", 10.0)],
        );

        assert!(result.is_err(), "\"percent\" no es un tipo de tasa válido");
    }

    #[test]
    fn update_rates_rejects_negative_value() {
        let mut conn = open_test_db();
        seed_employee(&conn);

        let result = update_employee_commission_rates(
            &mut conn,
            "emp-1".to_string(),
            vec![rate_input(None, "fixed", -5.0)],
        );

        assert!(result.is_err(), "la comisión no puede ser negativa");
    }

    #[test]
    fn update_rates_rejects_percentage_above_100() {
        let mut conn = open_test_db();
        seed_employee(&conn);

        let result = update_employee_commission_rates(
            &mut conn,
            "emp-1".to_string(),
            vec![rate_input(None, "percentage", 150.0)],
        );

        assert!(result.is_err(), "un porcentaje no puede superar 100");
    }

    #[test]
    fn update_rates_rejects_duplicate_scope() {
        let mut conn = open_test_db();
        seed_employee(&conn);
        seed_service(&conn);

        let result = update_employee_commission_rates(
            &mut conn,
            "emp-1".to_string(),
            vec![
                rate_input(Some("svc-1"), "percentage", 10.0),
                rate_input(Some("svc-1"), "fixed", 5.0),
            ],
        );

        assert!(result.is_err(), "no puede haber dos tasas para el mismo servicio");
    }

    #[test]
    fn update_rates_rejects_unknown_employee() {
        let mut conn = open_test_db();

        let result = update_employee_commission_rates(
            &mut conn,
            "missing-emp".to_string(),
            vec![rate_input(None, "percentage", 10.0)],
        );

        assert!(result.is_err(), "no debe poder configurar comisión de un empleado inexistente");
    }

    #[test]
    fn update_rates_replaces_previous_rates() {
        let mut conn = open_test_db();
        seed_employee(&conn);
        seed_service(&conn);

        update_employee_commission_rates(
            &mut conn,
            "emp-1".to_string(),
            vec![rate_input(None, "percentage", 10.0)],
        )
        .unwrap();

        let second = update_employee_commission_rates(
            &mut conn,
            "emp-1".to_string(),
            vec![rate_input(Some("svc-1"), "fixed", 5.0)],
        )
        .unwrap();

        assert_eq!(second.len(), 1, "la tasa general debe haberse reemplazado");
        assert_eq!(second[0].service_id.as_deref(), Some("svc-1"));
    }

    #[test]
    fn resolve_rate_prefers_service_override_over_default() {
        let mut map: HashMap<RateKey, RateValue> = HashMap::new();
        map.insert(("emp-1".to_string(), None), ("percentage".to_string(), 10.0));
        map.insert(
            ("emp-1".to_string(), Some("svc-1".to_string())),
            ("fixed".to_string(), 5.0),
        );

        let rate = resolve_rate(&map, "emp-1", Some("svc-1")).unwrap();
        assert_eq!(rate, &("fixed".to_string(), 5.0));
    }

    #[test]
    fn resolve_rate_falls_back_to_default_without_override() {
        let mut map: HashMap<RateKey, RateValue> = HashMap::new();
        map.insert(("emp-1".to_string(), None), ("percentage".to_string(), 10.0));

        let rate = resolve_rate(&map, "emp-1", Some("svc-1")).unwrap();
        assert_eq!(rate, &("percentage".to_string(), 10.0));
    }

    #[test]
    fn resolve_rate_none_when_unconfigured() {
        let map: HashMap<RateKey, RateValue> = HashMap::new();
        assert!(resolve_rate(&map, "emp-1", Some("svc-1")).is_none());
    }

    #[test]
    fn commission_amount_percentage_applies_over_sale_total() {
        let rate = ("percentage".to_string(), 10.0);
        assert_eq!(commission_amount(&rate, 200), 20.0);
    }

    #[test]
    fn commission_amount_fixed_ignores_sale_total() {
        let rate = ("fixed".to_string(), 7.5);
        assert_eq!(commission_amount(&rate, 200), 7.5);
    }
}

pub mod postgres {
    use std::collections::HashMap;

    use sqlx::{PgPool, Row};

    use crate::commands::commissions::CommissionRateInput;
    use crate::db::new_id;
    use crate::error::{AppError, AppResult};
    use crate::models::commission::EmployeeCommissionRate;

    use super::{validate_no_duplicate_scopes, validate_rate, RateKey, RateValue};

    pub async fn list_employee_commission_rates(
        pool: &PgPool,
        employee_id: String,
    ) -> AppResult<Vec<EmployeeCommissionRate>> {
        let rows = sqlx::query(
            "SELECT * FROM employee_commission_rates WHERE employee_id = $1
             ORDER BY service_id IS NULL DESC, service_id ASC",
        )
        .bind(employee_id)
        .fetch_all(pool)
        .await?;
        rows.iter().map(EmployeeCommissionRate::from_pg_row).collect()
    }

    pub async fn update_employee_commission_rates(
        pool: &PgPool,
        employee_id: String,
        rates: Vec<CommissionRateInput>,
    ) -> AppResult<Vec<EmployeeCommissionRate>> {
        for entry in &rates {
            validate_rate(&entry.rate_type, entry.rate_value)?;
        }
        validate_no_duplicate_scopes(&rates)?;

        let mut tx = pool.begin().await?;

        let employee_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM employees WHERE id = $1)")
                .bind(&employee_id)
                .fetch_one(&mut *tx)
                .await?;
        if !employee_exists {
            return Err(AppError::validation("El empleado no existe"));
        }

        sqlx::query("DELETE FROM employee_commission_rates WHERE employee_id = $1")
            .bind(&employee_id)
            .execute(&mut *tx)
            .await?;
        for entry in &rates {
            sqlx::query(
                "INSERT INTO employee_commission_rates (id, employee_id, service_id, rate_type, rate_value)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(new_id())
            .bind(&employee_id)
            .bind(&entry.service_id)
            .bind(&entry.rate_type)
            .bind(entry.rate_value)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        list_employee_commission_rates(pool, employee_id).await
    }

    pub async fn load_rate_map(pool: &PgPool) -> AppResult<HashMap<RateKey, RateValue>> {
        let rows =
            sqlx::query("SELECT employee_id, service_id, rate_type, rate_value FROM employee_commission_rates")
                .fetch_all(pool)
                .await?;
        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            let employee_id: String = row.try_get("employee_id")?;
            let service_id: Option<String> = row.try_get("service_id")?;
            let rate_type: String = row.try_get("rate_type")?;
            let rate_value: f64 = row.try_get("rate_value")?;
            map.insert((employee_id, service_id), (rate_type, rate_value));
        }
        Ok(map)
    }
}
