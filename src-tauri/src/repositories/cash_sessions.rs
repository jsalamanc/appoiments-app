//! Apertura y cierre de caja (issue #2, Fase 5). Ver
//! `commands::cash_sessions` para la convención de la API.

use rusqlite::{params, Connection, OptionalExtension};

use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::cash_session::CashSession;

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn get_current(conn: &mut Connection) -> AppResult<Option<CashSession>> {
    let result = conn.query_row(
        "SELECT * FROM cash_sessions WHERE closed_at IS NULL ORDER BY opened_at DESC LIMIT 1",
        [],
        CashSession::from_row,
    );
    match result {
        Ok(session) => Ok(Some(session)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn list(conn: &mut Connection, limit: i64) -> AppResult<Vec<CashSession>> {
    let mut stmt =
        conn.prepare("SELECT * FROM cash_sessions ORDER BY opened_at DESC LIMIT ?1")?;
    let rows = stmt
        .query_map(params![limit], CashSession::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Rechaza si ya hay una caja abierta — solo puede haber un turno activo a
/// la vez (ver el comentario de diseño en la migración `0006_cash_sessions`).
pub fn open(
    conn: &mut Connection,
    opened_by: Option<String>,
    opening_amount: f64,
) -> AppResult<CashSession> {
    if opening_amount < 0.0 {
        return Err(AppError::validation("El monto de apertura no puede ser negativo"));
    }

    let tx = conn.transaction()?;

    let already_open: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM cash_sessions WHERE closed_at IS NULL)",
        [],
        |r| r.get(0),
    )?;
    if already_open {
        return Err(AppError::validation("Ya hay una caja abierta"));
    }

    let id = new_id();
    let now = now_iso();
    tx.execute(
        "INSERT INTO cash_sessions (id, opened_by, opened_at, opening_amount)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, opened_by, now, opening_amount],
    )?;
    tx.commit()?;

    let session = conn.query_row(
        "SELECT * FROM cash_sessions WHERE id = ?1",
        params![id],
        CashSession::from_row,
    )?;
    Ok(session)
}

/// `expected_amount` = monto de apertura + ventas en efectivo (no anuladas)
/// registradas desde que se abrió el turno; `difference` = contado -
/// esperado. Ambos quedan fijos al cerrar, no se recalculan después.
pub fn close(
    conn: &mut Connection,
    id: String,
    closed_by: Option<String>,
    counted_amount: f64,
    notes: Option<String>,
) -> AppResult<CashSession> {
    if counted_amount < 0.0 {
        return Err(AppError::validation("El monto contado no puede ser negativo"));
    }

    let tx = conn.transaction()?;

    let (opened_at, opening_amount, already_closed): (String, f64, bool) = tx
        .query_row(
            "SELECT opened_at, opening_amount, closed_at IS NOT NULL FROM cash_sessions WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    if already_closed {
        return Err(AppError::validation("Esta caja ya fue cerrada"));
    }

    let cash_sales: f64 = tx.query_row(
        "SELECT COALESCE(SUM(sp.amount), 0.0) FROM sale_payments sp
         JOIN sales s ON s.id = sp.sale_id
         WHERE sp.method = 'cash' AND s.voided_at IS NULL AND s.sold_at >= ?1",
        params![opened_at],
        |r| r.get(0),
    )?;
    let expected_amount = round2(opening_amount + cash_sales);
    let difference = round2(counted_amount - expected_amount);
    let now = now_iso();

    tx.execute(
        "UPDATE cash_sessions SET
            closed_by = ?1, closed_at = ?2, counted_amount = ?3,
            expected_amount = ?4, difference = ?5, notes = ?6
         WHERE id = ?7",
        params![closed_by, now, counted_amount, expected_amount, difference, notes, id],
    )?;
    tx.commit()?;

    let session = conn.query_row(
        "SELECT * FROM cash_sessions WHERE id = ?1",
        params![id],
        CashSession::from_row,
    )?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations().to_latest(&mut conn).unwrap();
        conn
    }

    fn seed_customer(conn: &Connection) {
        conn.execute(
            "INSERT INTO customers (id, name, phone) VALUES ('cust-1', 'Cliente', '3000000000')",
            [],
        )
        .unwrap();
    }

    fn seed_cash_sale(conn: &mut Connection, total: f64) {
        let sale_id = new_id();
        conn.execute(
            "INSERT INTO sales (id, customer_id, payment_method, total) VALUES (?1, 'cust-1', 'cash', ?2)",
            params![sale_id, total],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sale_payments (id, sale_id, method, amount) VALUES (?1, ?2, 'cash', ?3)",
            params![new_id(), sale_id, total],
        )
        .unwrap();
    }

    #[test]
    fn open_rejects_negative_opening_amount() {
        let mut conn = open_test_db();

        let result = open(&mut conn, None, -1.0);

        assert!(result.is_err());
    }

    #[test]
    fn open_rejects_a_second_session_while_one_is_open() {
        let mut conn = open_test_db();
        open(&mut conn, None, 50.0).unwrap();

        let result = open(&mut conn, None, 100.0);

        assert!(result.is_err(), "no puede haber dos cajas abiertas a la vez");
    }

    #[test]
    fn get_current_returns_none_without_an_open_session() {
        let mut conn = open_test_db();

        assert!(get_current(&mut conn).unwrap().is_none());
    }

    #[test]
    fn get_current_returns_the_open_session() {
        let mut conn = open_test_db();
        let opened = open(&mut conn, None, 50.0).unwrap();

        let current = get_current(&mut conn).unwrap().unwrap();

        assert_eq!(current.id, opened.id);
    }

    #[test]
    fn close_computes_expected_amount_and_difference() {
        let mut conn = open_test_db();
        seed_customer(&conn);
        let session = open(&mut conn, None, 100.0).unwrap();
        seed_cash_sale(&mut conn, 50.0);
        seed_cash_sale(&mut conn, 30.0);

        let closed = close(&mut conn, session.id, None, 175.0, None).unwrap();

        assert_eq!(closed.expected_amount, Some(180.0), "100 de apertura + 50 + 30 en efectivo");
        assert_eq!(closed.counted_amount, Some(175.0));
        assert_eq!(closed.difference, Some(-5.0), "contado - esperado = 175 - 180");
        assert!(get_current(&mut conn).unwrap().is_none(), "ya no debe quedar ninguna caja abierta");
    }

    #[test]
    fn close_ignores_voided_sales() {
        let mut conn = open_test_db();
        seed_customer(&conn);
        let session = open(&mut conn, None, 0.0).unwrap();
        seed_cash_sale(&mut conn, 50.0);
        let voided_sale_id = new_id();
        conn.execute(
            "INSERT INTO sales (id, customer_id, payment_method, total, voided_at)
             VALUES (?1, 'cust-1', 'cash', 999.0, '2026-01-01T00:00:00Z')",
            params![voided_sale_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sale_payments (id, sale_id, method, amount) VALUES (?1, ?2, 'cash', 999.0)",
            params![new_id(), voided_sale_id],
        )
        .unwrap();

        let closed = close(&mut conn, session.id, None, 50.0, None).unwrap();

        assert_eq!(closed.expected_amount, Some(50.0), "la venta anulada no debe contar");
    }

    #[test]
    fn close_rejects_unknown_session() {
        let mut conn = open_test_db();

        let result = close(&mut conn, "missing".to_string(), None, 0.0, None);

        assert!(result.is_err());
    }

    #[test]
    fn close_rejects_already_closed_session() {
        let mut conn = open_test_db();
        let session = open(&mut conn, None, 0.0).unwrap();
        close(&mut conn, session.id.clone(), None, 0.0, None).unwrap();

        let result = close(&mut conn, session.id, None, 0.0, None);

        assert!(result.is_err(), "no se puede cerrar una caja dos veces");
    }
}

pub mod postgres {
    use sqlx::PgPool;

    use crate::db::new_id;
    use crate::error::{AppError, AppResult};
    use crate::models::cash_session::CashSession;

    use super::round2;

    pub async fn get_current(pool: &PgPool) -> AppResult<Option<CashSession>> {
        let row = sqlx::query(
            "SELECT * FROM cash_sessions WHERE closed_at IS NULL ORDER BY opened_at DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;
        row.as_ref().map(CashSession::from_pg_row).transpose()
    }

    pub async fn list(pool: &PgPool, limit: i64) -> AppResult<Vec<CashSession>> {
        let rows = sqlx::query("SELECT * FROM cash_sessions ORDER BY opened_at DESC LIMIT $1")
            .bind(limit)
            .fetch_all(pool)
            .await?;
        rows.iter().map(CashSession::from_pg_row).collect()
    }

    pub async fn open(
        pool: &PgPool,
        opened_by: Option<String>,
        opening_amount: f64,
    ) -> AppResult<CashSession> {
        if opening_amount < 0.0 {
            return Err(AppError::validation("El monto de apertura no puede ser negativo"));
        }

        let mut tx = pool.begin().await?;

        let already_open: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM cash_sessions WHERE closed_at IS NULL)",
        )
        .fetch_one(&mut *tx)
        .await?;
        if already_open {
            return Err(AppError::validation("Ya hay una caja abierta"));
        }

        let row = sqlx::query(
            "INSERT INTO cash_sessions (id, opened_by, opening_amount)
             VALUES ($1, $2, $3)
             RETURNING *",
        )
        .bind(new_id())
        .bind(&opened_by)
        .bind(opening_amount)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        CashSession::from_pg_row(&row)
    }

    pub async fn close(
        pool: &PgPool,
        id: String,
        closed_by: Option<String>,
        counted_amount: f64,
        notes: Option<String>,
    ) -> AppResult<CashSession> {
        if counted_amount < 0.0 {
            return Err(AppError::validation("El monto contado no puede ser negativo"));
        }

        let mut tx = pool.begin().await?;

        let existing = sqlx::query(
            "SELECT opened_at, opening_amount, closed_at IS NOT NULL AS already_closed
             FROM cash_sessions WHERE id = $1",
        )
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;

        use sqlx::Row;
        let opened_at: time::OffsetDateTime = existing.try_get("opened_at")?;
        let opening_amount: f64 = existing.try_get("opening_amount")?;
        let already_closed: bool = existing.try_get("already_closed")?;
        if already_closed {
            return Err(AppError::validation("Esta caja ya fue cerrada"));
        }

        let cash_sales: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(sp.amount), 0.0) FROM sale_payments sp
             JOIN sales s ON s.id = sp.sale_id
             WHERE sp.method = 'cash' AND s.voided_at IS NULL AND s.sold_at >= $1",
        )
        .bind(opened_at)
        .fetch_one(&mut *tx)
        .await?;
        let expected_amount = round2(opening_amount + cash_sales);
        let difference = round2(counted_amount - expected_amount);

        let row = sqlx::query(
            "UPDATE cash_sessions SET
                closed_by = $1, closed_at = now(), counted_amount = $2,
                expected_amount = $3, difference = $4, notes = $5
             WHERE id = $6
             RETURNING *",
        )
        .bind(&closed_by)
        .bind(counted_amount)
        .bind(expected_amount)
        .bind(difference)
        .bind(&notes)
        .bind(&id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        CashSession::from_pg_row(&row)
    }
}
