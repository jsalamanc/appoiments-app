//! Bitácora de auditoría: quién hizo qué, cuándo y qué cambió exactamente
//! (ventas, anulaciones, inventario, catálogo, citas, clientes, empleados,
//! caja, usuarios, configuración, inicios de sesión).
//!
//! Dos formas de usarla desde un comando de Tauri:
//!
//! * [`Tracker`] para cambios sobre una fila de la base de datos: toma una
//!   foto de la fila **antes** de ejecutar la acción y otra **después**, y
//!   guarda las dos junto con el diff campo por campo. Funciona igual en
//!   SQLite y PostgreSQL porque lee las filas de forma genérica
//!   (`SELECT *` / `to_jsonb`), sin depender de los modelos de cada dominio.
//! * [`record`] para eventos que no son un cambio de fila (inicio de sesión,
//!   cambio de modo de almacenamiento, etc.).
//!
//! La tabla `audit_log` es solo de inserción (triggers rechazan UPDATE y
//! DELETE) y no tiene llaves foráneas: guarda copia del nombre del actor y
//! el id de la entidad para que el rastro sobreviva aunque se borre el
//! usuario o el registro afectado.
//!
//! Limitación conocida: la bitácora se escribe justo DESPUÉS de que la
//! acción se confirma, en una escritura aparte (no en la misma transacción
//! que el repositorio). Si esa escritura fallara, la acción ya ocurrió y el
//! error se reporta por stderr sin deshacerla ni mostrarle al usuario un
//! error engañoso.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::app_db::AppDb;
use crate::db::{format_pg_timestamp, new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::user::UserPublic;
use crate::util::pagination::{get_range, get_total_pages};

/// Campos que no cuentan como "cambio" en el diff: se actualizan solos.
const IGNORED_DIFF_FIELDS: [&str; 2] = ["updated_at", "created_at"];

/// Campos que nunca deben quedar escritos en la bitácora.
const REDACTED_FIELDS: [&str; 1] = ["password_hash"];

/// Para entidades que son una tabla entera (ej. el horario del negocio):
/// `Tracker::start_by(.., ALL_ROWS.0, Some(ALL_ROWS.1), ..)` captura todas las
/// filas (`WHERE 1 = '1'`).
pub const ALL_ROWS: (&str, &str) = ("1", "1");

/// Tablas hijas a incluir dentro de la foto de la fila principal:
/// `(tabla, columna que apunta a la fila principal)`.
pub type Related = &'static [(&'static str, &'static str)];

pub struct NewEntry {
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub summary: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub changes: Option<Value>,
}

impl NewEntry {
    /// Evento simple sin foto antes/después. `entity_type` sale del prefijo
    /// de `action` (`"sale.void"` → `"sale"`).
    pub fn event(action: &str, entity_id: Option<&str>, summary: impl Into<String>) -> Self {
        Self {
            action: action.to_string(),
            entity_type: entity_type_of(action),
            entity_id: entity_id.map(str::to_string),
            summary: summary.into(),
            before: None,
            after: None,
            changes: None,
        }
    }

    pub fn with_after(mut self, after: Value) -> Self {
        self.after = Some(after);
        self
    }
}

fn entity_type_of(action: &str) -> String {
    action.split('.').next().unwrap_or(action).to_string()
}

// ---------------------------------------------------------------------------
// Lectura genérica de filas (para las fotos antes/después)
// ---------------------------------------------------------------------------

/// `table` y `col` siempre son literales fijos escritos en el código que
/// llama, nunca datos del usuario.
async fn fetch_rows(db: &AppDb, table: &'static str, col: &'static str, key: &str) -> AppResult<Vec<Value>> {
    match db {
        AppDb::Sqlite(_) => {
            let key = key.to_string();
            db.with_sqlite(move |conn| {
                let mut stmt = conn.prepare(&format!(
                    "SELECT * FROM {table} WHERE CAST({col} AS TEXT) = ?1 ORDER BY 1"
                ))?;
                let columns: Vec<String> =
                    stmt.column_names().into_iter().map(|s| s.to_string()).collect();
                let rows = stmt
                    .query_map(rusqlite::params![key], |row| {
                        crate::export::row_to_json(row, &columns)
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows.into_iter().map(Value::Object).collect())
            })
            .await
        }
        AppDb::Postgres(pool) => {
            let rows: Vec<String> = sqlx::query_scalar(&format!(
                "SELECT to_jsonb(t)::text FROM {table} t WHERE {col}::text = $1 ORDER BY 1"
            ))
            .bind(key)
            .fetch_all(pool)
            .await?;
            rows.iter()
                .map(|r| {
                    serde_json::from_str(r)
                        .map_err(|e| AppError::Internal(format!("fila de auditoría inválida: {e}")))
                })
                .collect()
        }
    }
}

fn redact(value: &mut Value) {
    if let Some(obj) = value.as_object_mut() {
        for field in REDACTED_FIELDS {
            obj.remove(field);
        }
    }
}

/// Foto de lo que hay en `table` donde `col = key`. Con `col == "id"` es la
/// fila (más sus tablas hijas en `related`); con otra columna es el arreglo
/// de todas las filas que coinciden. `None` si no hay ninguna.
async fn capture(
    db: &AppDb,
    table: &'static str,
    col: &'static str,
    key: &str,
    related: Related,
) -> Option<Value> {
    let rows = match fetch_rows(db, table, col, key).await {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("[audit] no se pudo leer {table} para la bitácora: {e}");
            return None;
        }
    };
    if rows.is_empty() {
        return None;
    }
    if col != "id" {
        return Some(Value::Array(rows));
    }
    let mut row = rows.into_iter().next()?;
    redact(&mut row);
    if table == "sales" {
        let customer_id = row.get("customer_id").and_then(Value::as_str).map(str::to_string);
        if let Some(name) = lookup_name(db, "customers", customer_id.as_deref()).await {
            if let Some(obj) = row.as_object_mut() {
                obj.insert("customer_name".into(), Value::String(name));
            }
        }
    }
    for (child_table, child_col) in related {
        let mut children = fetch_rows(db, child_table, child_col, key).await.unwrap_or_default();
        children.iter_mut().for_each(redact);
        if *child_table == "inventory_movements" {
            for child in children.iter_mut() {
                let product_id = child.get("product_id").and_then(Value::as_str).map(str::to_string);
                if let Some(name) = lookup_name(db, "products", product_id.as_deref()).await {
                    if let Some(obj) = child.as_object_mut() {
                        obj.insert("product_name".into(), Value::String(name));
                    }
                }
            }
        }
        if let Some(obj) = row.as_object_mut() {
            obj.insert((*child_table).to_string(), Value::Array(children));
        }
    }
    Some(row)
}

/// Nombre legible de una fila (para armar resúmenes de entidades hijas, ej.
/// el empleado dueño de unas comisiones). Vacío si no existe.
pub async fn name_of(db: &AppDb, table: &'static str, id: &str) -> String {
    let snapshot = capture(db, table, "id", id, &[]).await;
    match snapshot.as_ref().and_then(|v| v.get("name")).and_then(Value::as_str) {
        Some(name) => name.to_string(),
        None => String::new(),
    }
}

/// Diff campo por campo: `{campo: {from, to}}` solo con lo que cambió.
pub fn diff(before: &Value, after: &Value) -> Value {
    match (before, after) {
        (Value::Object(b), Value::Object(a)) => {
            let mut out = Map::new();
            for key in b.keys().chain(a.keys()) {
                if IGNORED_DIFF_FIELDS.contains(&key.as_str()) || out.contains_key(key) {
                    continue;
                }
                let from = b.get(key).unwrap_or(&Value::Null);
                let to = a.get(key).unwrap_or(&Value::Null);
                if from != to {
                    out.insert(key.clone(), json!({ "from": from, "to": to }));
                }
            }
            Value::Object(out)
        }
        (Value::Array(b), Value::Array(a)) if b != a => {
            json!({ "rows": { "from": b, "to": a } })
        }
        (Value::Array(_), Value::Array(_)) => json!({}),
        _ => Value::Null,
    }
}

/// `name` de la fila `id` de `table`, si existe (para guardar el nombre junto
/// al id y que el registro se entienda sin cruzar tablas).
async fn lookup_name(db: &AppDb, table: &'static str, id: Option<&str>) -> Option<String> {
    let id = id?;
    let rows = fetch_rows(db, table, "id", id).await.ok()?;
    rows.first()?.get("name")?.as_str().map(str::to_string)
}

/// Resumen de las líneas de una venta: "Ana: 2× Shampoo, 1× Corte".
fn sale_detail(snapshot: Option<&Value>) -> Option<String> {
    let obj = snapshot?.as_object()?;
    let lines: Vec<String> = obj
        .get("sale_items")?
        .as_array()?
        .iter()
        .filter_map(|item| {
            let name = item.get("name_snapshot")?.as_str()?;
            let qty = item.get("quantity").and_then(Value::as_i64).unwrap_or(1);
            Some(format!("{qty}× {name}"))
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    let items_text = lines.join(", ");
    match obj.get("customer_name").and_then(Value::as_str) {
        Some(customer) => Some(format!("{customer}: {items_text}")),
        None => Some(items_text),
    }
}

fn label_of(snapshot: Option<&Value>, fallback_id: &str) -> String {
    if let Some(obj) = snapshot.and_then(Value::as_object) {
        for key in ["name", "name_snapshot", "username"] {
            if let Some(text) = obj.get(key).and_then(Value::as_str) {
                if !text.trim().is_empty() {
                    return text.to_string();
                }
            }
        }
    }
    fallback_id.chars().take(8).collect()
}

// ---------------------------------------------------------------------------
// Escritura
// ---------------------------------------------------------------------------

struct Row {
    id: String,
    actor_user_id: Option<String>,
    actor_name: Option<String>,
    actor_username: Option<String>,
    actor_role: Option<String>,
    action: String,
    entity_type: String,
    entity_id: Option<String>,
    summary: String,
    before_json: Option<String>,
    after_json: Option<String>,
    changes_json: Option<String>,
}

fn to_json_text(value: &Option<Value>) -> Option<String> {
    value.as_ref().map(|v| v.to_string())
}

/// Inserta la entrada y devuelve el error si falla. La mayoría de los
/// llamadores quieren [`record`], que no propaga el error.
pub async fn insert(db: &AppDb, actor: Option<&UserPublic>, entry: NewEntry) -> AppResult<()> {
    let row = Row {
        id: new_id(),
        actor_user_id: actor.map(|a| a.id.clone()),
        actor_name: actor.map(|a| a.name.clone()),
        actor_username: actor.map(|a| a.username.clone()),
        actor_role: actor.map(|a| a.role.clone()),
        action: entry.action,
        entity_type: entry.entity_type,
        entity_id: entry.entity_id,
        summary: entry.summary,
        before_json: to_json_text(&entry.before),
        after_json: to_json_text(&entry.after),
        changes_json: to_json_text(&entry.changes),
    };

    match db {
        AppDb::Sqlite(_) => {
            db.with_sqlite(move |conn| {
                conn.execute(
                    "INSERT INTO audit_log
                        (id, occurred_at, actor_user_id, actor_name, actor_username, actor_role,
                         action, entity_type, entity_id, summary, before_json, after_json,
                         changes_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    rusqlite::params![
                        row.id,
                        now_iso(),
                        row.actor_user_id,
                        row.actor_name,
                        row.actor_username,
                        row.actor_role,
                        row.action,
                        row.entity_type,
                        row.entity_id,
                        row.summary,
                        row.before_json,
                        row.after_json,
                        row.changes_json,
                    ],
                )?;
                Ok(())
            })
            .await
        }
        AppDb::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO audit_log
                    (id, actor_user_id, actor_name, actor_username, actor_role, action,
                     entity_type, entity_id, summary, before_json, after_json, changes_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            )
            .bind(row.id)
            .bind(row.actor_user_id)
            .bind(row.actor_name)
            .bind(row.actor_username)
            .bind(row.actor_role)
            .bind(row.action)
            .bind(row.entity_type)
            .bind(row.entity_id)
            .bind(row.summary)
            .bind(row.before_json)
            .bind(row.after_json)
            .bind(row.changes_json)
            .execute(pool)
            .await?;
            Ok(())
        }
    }
}

/// Registra la entrada sin propagar errores (ver la nota de limitación al
/// inicio del archivo).
pub async fn record(db: &AppDb, actor: Option<&UserPublic>, entry: NewEntry) {
    let action = entry.action.clone();
    if let Err(e) = insert(db, actor, entry).await {
        eprintln!("[audit] no se pudo registrar '{action}': {e}");
    }
}

/// Sigue un cambio sobre una fila: foto antes (en `start`) y después (en
/// `finish*`), con el diff entre ambas.
pub struct Tracker<'a> {
    db: &'a AppDb,
    actor: UserPublic,
    action: &'static str,
    table: &'static str,
    col: &'static str,
    key: Option<String>,
    related: Related,
    before: Option<Value>,
    detail: Option<String>,
}

impl<'a> Tracker<'a> {
    /// Fila identificada por `id`. `id = None` para creaciones (todavía no se
    /// conoce el id; se pasa a [`Tracker::finish_created`]).
    pub async fn start(
        db: &'a AppDb,
        actor: &UserPublic,
        action: &'static str,
        table: &'static str,
        id: Option<&str>,
        related: Related,
    ) -> Self {
        Self::start_by(db, actor, action, table, "id", id, related).await
    }

    /// Variante para entidades que ocupan varias filas identificadas por otra
    /// columna (ej. las comisiones de un empleado: `employee_id`).
    pub async fn start_by(
        db: &'a AppDb,
        actor: &UserPublic,
        action: &'static str,
        table: &'static str,
        col: &'static str,
        key: Option<&str>,
        related: Related,
    ) -> Self {
        let before = match key {
            Some(key) => capture(db, table, col, key, related).await,
            None => None,
        };
        Self {
            db,
            actor: actor.clone(),
            action,
            table,
            col,
            key: key.map(str::to_string),
            related,
            before,
            detail: None,
        }
    }

    /// Texto extra para el resumen (ej. la nota de un ajuste de inventario).
    pub fn detail(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        if !text.trim().is_empty() {
            self.detail = Some(text);
        }
        self
    }

    /// El cambio ocurrió sobre la fila con la que se inició.
    pub async fn finish(self, verb: &str) {
        let id = self.key.clone();
        self.complete(verb, id, false).await;
    }

    /// Creación: el id solo se conoce una vez que la acción terminó.
    pub async fn finish_created(self, verb: &str, new_id: &str) {
        self.complete(verb, Some(new_id.to_string()), false).await;
    }

    /// Borrado: la foto de "antes" conserva todo lo que se eliminó.
    pub async fn finish_deleted(self, verb: &str) {
        let id = self.key.clone();
        self.complete(verb, id, true).await;
    }

    async fn complete(self, verb: &str, entity_id: Option<String>, deleted: bool) {
        let entity_id_text = entity_id.clone().unwrap_or_default();
        let after = if deleted {
            None
        } else {
            capture(self.db, self.table, self.col, &entity_id_text, self.related).await
        };
        let changes = match (&self.before, &after) {
            (Some(b), Some(a)) => Some(diff(b, a)),
            _ => None,
        };
        let label = label_of(after.as_ref().or(self.before.as_ref()), &entity_id_text);
        let detail = self.detail.clone().or_else(|| {
            if self.table == "sales" {
                sale_detail(after.as_ref().or(self.before.as_ref()))
            } else {
                None
            }
        });
        let entry = NewEntry {
            action: self.action.to_string(),
            entity_type: entity_type_of(self.action),
            entity_id,
            summary: match &detail {
                Some(detail) => format!("{verb} «{label}» — {detail}"),
                None => format!("{verb} «{label}»"),
            },
            before: self.before,
            after,
            changes,
        };
        record(self.db, Some(&self.actor), entry).await;
    }
}

// ---------------------------------------------------------------------------
// Consulta (pantalla de auditoría del dueño)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct AuditFilters {
    pub from: Option<String>,
    pub to: Option<String>,
    pub actor_user_id: Option<String>,
    pub entity_type: Option<String>,
    pub action: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: String,
    pub occurred_at: String,
    pub actor_user_id: Option<String>,
    pub actor_name: Option<String>,
    pub actor_username: Option<String>,
    pub actor_role: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub summary: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub changes: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ListAuditResult {
    pub items: Vec<AuditEntry>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
}

pub const AUDIT_PAGE_SIZE: i64 = 25;
const MAX_AUDIT_PAGE_SIZE: i64 = 500;

fn parse_json_text(text: Option<String>) -> Option<Value> {
    text.and_then(|t| serde_json::from_str(&t).ok())
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn like_pattern(search: &str) -> String {
    format!("%{}%", search.to_lowercase())
}

pub async fn list(db: &AppDb, filters: AuditFilters) -> AppResult<ListAuditResult> {
    let page = filters.page.unwrap_or(1).max(1);
    let page_size = filters
        .page_size
        .unwrap_or(AUDIT_PAGE_SIZE)
        .clamp(1, MAX_AUDIT_PAGE_SIZE);

    match db {
        AppDb::Sqlite(_) => {
            db.with_sqlite(move |conn| list_sqlite(conn, filters, page, page_size))
                .await
        }
        AppDb::Postgres(pool) => list_postgres(pool, filters, page, page_size).await,
    }
}

const SELECT_COLUMNS: &str = "id, occurred_at, actor_user_id, actor_name, actor_username, \
     actor_role, action, entity_type, entity_id, summary, before_json, after_json, changes_json";

fn list_sqlite(
    conn: &mut rusqlite::Connection,
    filters: AuditFilters,
    page: i64,
    page_size: i64,
) -> AppResult<ListAuditResult> {
    let mut clauses: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    let mut push = |clause: &str, value: String| {
        args.push(value);
        clauses.push(clause.replace('?', &format!("?{}", args.len())));
    };

    if let Some(v) = clean(&filters.from) {
        push("occurred_at >= ?", v);
    }
    if let Some(v) = clean(&filters.to) {
        push("occurred_at <= ?", v);
    }
    if let Some(v) = clean(&filters.actor_user_id) {
        push("actor_user_id = ?", v);
    }
    if let Some(v) = clean(&filters.entity_type) {
        push("entity_type = ?", v);
    }
    if let Some(v) = clean(&filters.action) {
        push("action = ?", v);
    }
    if let Some(v) = clean(&filters.search) {
        let n = args.len() + 1;
        args.push(like_pattern(&v));
        clauses.push(format!(
            "(lower(summary) LIKE ?{n} OR lower(COALESCE(actor_name, '')) LIKE ?{n} \
             OR lower(COALESCE(actor_username, '')) LIKE ?{n} \
             OR lower(COALESCE(entity_id, '')) LIKE ?{n})"
        ));
    }

    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };

    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM audit_log {where_sql}"),
        rusqlite::params_from_iter(args.iter()),
        |r| r.get(0),
    )?;
    let total_pages = get_total_pages(total, page_size);
    let safe_page = page.min(total_pages);
    let (limit, offset) = get_range(safe_page, page_size);

    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM audit_log {where_sql}
         ORDER BY occurred_at DESC, id DESC LIMIT {limit} OFFSET {offset}"
    ))?;
    let items = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), |row| {
            Ok(AuditEntry {
                id: row.get("id")?,
                occurred_at: row.get("occurred_at")?,
                actor_user_id: row.get("actor_user_id")?,
                actor_name: row.get("actor_name")?,
                actor_username: row.get("actor_username")?,
                actor_role: row.get("actor_role")?,
                action: row.get("action")?,
                entity_type: row.get("entity_type")?,
                entity_id: row.get("entity_id")?,
                summary: row.get("summary")?,
                before: parse_json_text(row.get("before_json")?),
                after: parse_json_text(row.get("after_json")?),
                changes: parse_json_text(row.get("changes_json")?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListAuditResult {
        items,
        total,
        page: safe_page,
        total_pages,
        page_size,
    })
}

fn parse_ts(value: &str) -> AppResult<time::OffsetDateTime> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map_err(|_| AppError::validation("Fecha inválida en el filtro de auditoría"))
}

fn apply_pg_filters(
    qb: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>,
    filters: &AuditFilters,
) -> AppResult<()> {
    if let Some(v) = clean(&filters.from) {
        qb.push(" AND occurred_at >= ").push_bind(parse_ts(&v)?);
    }
    if let Some(v) = clean(&filters.to) {
        qb.push(" AND occurred_at <= ").push_bind(parse_ts(&v)?);
    }
    if let Some(v) = clean(&filters.actor_user_id) {
        qb.push(" AND actor_user_id = ").push_bind(v);
    }
    if let Some(v) = clean(&filters.entity_type) {
        qb.push(" AND entity_type = ").push_bind(v);
    }
    if let Some(v) = clean(&filters.action) {
        qb.push(" AND action = ").push_bind(v);
    }
    if let Some(v) = clean(&filters.search) {
        let pattern = like_pattern(&v);
        qb.push(" AND (lower(summary) LIKE ")
            .push_bind(pattern.clone())
            .push(" OR lower(COALESCE(actor_name, '')) LIKE ")
            .push_bind(pattern.clone())
            .push(" OR lower(COALESCE(actor_username, '')) LIKE ")
            .push_bind(pattern.clone())
            .push(" OR lower(COALESCE(entity_id, '')) LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    Ok(())
}

async fn list_postgres(
    pool: &sqlx::PgPool,
    filters: AuditFilters,
    page: i64,
    page_size: i64,
) -> AppResult<ListAuditResult> {
    use sqlx::Row as _;

    let mut count_qb = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM audit_log WHERE 1=1");
    apply_pg_filters(&mut count_qb, &filters)?;
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let total_pages = get_total_pages(total, page_size);
    let safe_page = page.min(total_pages);
    let (limit, offset) = get_range(safe_page, page_size);

    let mut qb = sqlx::QueryBuilder::new(format!(
        "SELECT {SELECT_COLUMNS} FROM audit_log WHERE 1=1"
    ));
    apply_pg_filters(&mut qb, &filters)?;
    qb.push(" ORDER BY occurred_at DESC, id DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let rows = qb.build().fetch_all(pool).await?;
    let items = rows
        .iter()
        .map(|row| {
            let occurred_at: time::OffsetDateTime = row.try_get("occurred_at")?;
            Ok(AuditEntry {
                id: row.try_get("id")?,
                occurred_at: format_pg_timestamp(occurred_at),
                actor_user_id: row.try_get("actor_user_id")?,
                actor_name: row.try_get("actor_name")?,
                actor_username: row.try_get("actor_username")?,
                actor_role: row.try_get("actor_role")?,
                action: row.try_get("action")?,
                entity_type: row.try_get("entity_type")?,
                entity_id: row.try_get("entity_id")?,
                summary: row.try_get("summary")?,
                before: parse_json_text(row.try_get("before_json")?),
                after: parse_json_text(row.try_get("after_json")?),
                changes: parse_json_text(row.try_get("changes_json")?),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    Ok(ListAuditResult {
        items,
        total,
        page: safe_page,
        total_pages,
        page_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations().to_latest(&mut conn).unwrap();
        conn
    }

    #[test]
    fn diff_reports_only_changed_fields() {
        let before = json!({"id": "p1", "name": "Shampoo", "price": 1000, "updated_at": "a"});
        let after = json!({"id": "p1", "name": "Shampoo", "price": 1500, "updated_at": "b"});
        let changes = diff(&before, &after);
        assert_eq!(changes, json!({"price": {"from": 1000, "to": 1500}}));
    }

    #[test]
    fn diff_detects_added_and_removed_fields() {
        let changes = diff(&json!({"a": 1}), &json!({"b": 2}));
        assert_eq!(
            changes,
            json!({"a": {"from": 1, "to": null}, "b": {"from": null, "to": 2}})
        );
    }

    #[test]
    fn redact_removes_password_hash() {
        let mut value = json!({"username": "x", "password_hash": "secret"});
        redact(&mut value);
        assert_eq!(value, json!({"username": "x"}));
    }

    #[test]
    fn audit_log_rejects_update_and_delete() {
        let conn = open_test_db();
        conn.execute(
            "INSERT INTO audit_log (id, action, entity_type, summary) VALUES ('a1', 'x.y', 'x', 's')",
            [],
        )
        .unwrap();
        assert!(conn
            .execute("UPDATE audit_log SET summary = 'cambiado' WHERE id = 'a1'", [])
            .is_err());
        assert!(conn.execute("DELETE FROM audit_log WHERE id = 'a1'", []).is_err());
    }

    #[test]
    fn list_filters_and_paginates() {
        let mut conn = open_test_db();
        for (i, action) in ["sale.create", "sale.void", "product.update"].iter().enumerate() {
            conn.execute(
                "INSERT INTO audit_log (id, occurred_at, action, entity_type, summary, actor_name)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'Ana')",
                rusqlite::params![
                    format!("id{i}"),
                    format!("2026-01-0{}T10:00:00Z", i + 1),
                    action,
                    entity_type_of(action),
                    format!("resumen {action}"),
                ],
            )
            .unwrap();
        }

        let all = list_sqlite(&mut conn, AuditFilters::default(), 1, 2).unwrap();
        assert_eq!(all.total, 3);
        assert_eq!(all.total_pages, 2);
        assert_eq!(all.items.len(), 2);
        assert_eq!(all.items[0].action, "product.update");

        let sales = list_sqlite(
            &mut conn,
            AuditFilters {
                entity_type: Some("sale".into()),
                ..Default::default()
            },
            1,
            25,
        )
        .unwrap();
        assert_eq!(sales.total, 2);

        let searched = list_sqlite(
            &mut conn,
            AuditFilters {
                search: Some("VOID".into()),
                ..Default::default()
            },
            1,
            25,
        )
        .unwrap();
        assert_eq!(searched.total, 1);
        assert_eq!(searched.items[0].action, "sale.void");
    }

    #[test]
    fn tracker_records_before_after_and_diff_end_to_end() {
        use std::sync::{Arc, Mutex};

        let conn = open_test_db();
        conn.execute(
            "INSERT INTO products (id, name, price, cost, stock, status)
             VALUES ('p1', 'Shampoo', 1000, 400, 5, 'published')",
            [],
        )
        .unwrap();
        let db = AppDb::Sqlite(Arc::new(Mutex::new(conn)));
        let actor = UserPublic {
            id: "u1".into(),
            name: "Ana".into(),
            username: "ana".into(),
            role: "staff".into(),
            active: true,
            created_at: String::new(),
            updated_at: String::new(),
        };

        tauri::async_runtime::block_on(async {
            let tracker =
                Tracker::start(&db, &actor, "product.update", "products", Some("p1"), &[]).await;
            db.with_sqlite(|conn| {
                conn.execute("UPDATE products SET price = 1500 WHERE id = 'p1'", [])?;
                Ok(())
            })
            .await
            .unwrap();
            tracker.finish("Actualizó producto").await;

            let result = list(&db, AuditFilters::default()).await.unwrap();
            assert_eq!(result.total, 1);
            let entry = &result.items[0];
            assert_eq!(entry.actor_username.as_deref(), Some("ana"));
            assert_eq!(entry.entity_id.as_deref(), Some("p1"));
            assert_eq!(entry.summary, "Actualizó producto «Shampoo»");
            assert_eq!(
                entry.changes,
                Some(json!({"price": {"from": 1000, "to": 1500}}))
            );
            assert_eq!(entry.before.as_ref().unwrap()["price"], json!(1000));
            assert_eq!(entry.after.as_ref().unwrap()["price"], json!(1500));
        });
    }

    #[test]
    fn sale_detail_lists_product_names_and_customer() {
        let sale = json!({
            "customer_name": "Ana",
            "sale_items": [
                {"name_snapshot": "Shampoo", "quantity": 2},
                {"name_snapshot": "Corte", "quantity": 1}
            ]
        });
        assert_eq!(
            sale_detail(Some(&sale)).as_deref(),
            Some("Ana: 2× Shampoo, 1× Corte")
        );
        assert_eq!(sale_detail(Some(&json!({"sale_items": []}))), None);
    }
}
