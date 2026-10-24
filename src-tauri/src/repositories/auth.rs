use rusqlite::{params, Connection, Row};

use crate::auth::{generate_session_token, hash_password, verify_password};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::user::UserPublic;

const SESSION_TTL_DAYS: i64 = 30;

fn row_to_user(row: &Row) -> rusqlite::Result<UserPublic> {
    Ok(UserPublic {
        id: row.get("id")?,
        name: row.get("name")?,
        username: row.get("username")?,
        role: row.get("role")?,
        active: row.get::<_, i64>("active")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Verifica usuario/contraseña y, si `remember_me`, crea una sesión. Devuelve
/// el usuario y el token de sesión (si corresponde) para que el comando
/// actualice `current_user` una vez liberado el lock de la conexión.
pub fn login(
    conn: &mut Connection,
    username: String,
    password: String,
    remember_me: bool,
) -> AppResult<(UserPublic, Option<String>)> {
    let username_norm = username.trim().to_lowercase();
    let mut stmt = conn.prepare("SELECT * FROM users WHERE lower(username) = ?1 AND active = 1")?;
    let user_row = stmt
        .query_row(params![username_norm], |row| {
            let user = row_to_user(row)?;
            let hash: String = row.get("password_hash")?;
            Ok((user, hash))
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::validation("Usuario o contraseña incorrectos")
            }
            other => AppError::from(other),
        })?;

    drop(stmt);
    let (user, password_hash) = user_row;

    if !verify_password(&password, &password_hash) {
        return Err(AppError::validation("Usuario o contraseña incorrectos"));
    }

    let session_token = if remember_me {
        let token = generate_session_token();
        let now = now_iso();
        let expires_at = time::OffsetDateTime::now_utc()
            .saturating_add(time::Duration::days(SESSION_TTL_DAYS))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or(now);
        conn.execute(
            "INSERT INTO sessions (token, user_id, expires_at) VALUES (?1, ?2, ?3)",
            params![token, user.id, expires_at],
        )?;
        Some(token)
    } else {
        None
    };

    Ok((user, session_token))
}

pub fn delete_session(conn: &mut Connection, token: String) -> AppResult<()> {
    conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
    Ok(())
}

/// Busca una sesión vigente y su usuario. Igual que la implementación
/// original: cualquier error de la consulta (no solo "sin filas") se trata
/// como sesión inválida, no como un error a propagar.
pub fn resume_session(conn: &mut Connection, session_token: String) -> AppResult<Option<UserPublic>> {
    let now = now_iso();
    let mut stmt = conn.prepare(
        "SELECT u.* FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.token = ?1 AND s.expires_at > ?2 AND u.active = 1",
    )?;
    let user = stmt
        .query_row(params![session_token, now], row_to_user)
        .ok();
    Ok(user)
}

pub fn change_password(
    conn: &mut Connection,
    user_id: String,
    old_password: Option<String>,
    new_password: String,
    verify_old_password: bool,
) -> AppResult<()> {
    if verify_old_password {
        let existing_hash: String = conn.query_row(
            "SELECT password_hash FROM users WHERE id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;
        let old = old_password.unwrap_or_default();
        if !verify_password(&old, &existing_hash) {
            return Err(AppError::validation("La contraseña actual no es correcta"));
        }
    }

    if new_password.len() < 6 {
        return Err(AppError::validation(
            "La nueva contraseña debe tener al menos 6 caracteres",
        ));
    }

    let new_hash = hash_password(&new_password)?;
    conn.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
        params![new_hash, now_iso(), user_id],
    )?;

    Ok(())
}

pub fn create_staff(
    conn: &mut Connection,
    name: String,
    username: String,
    password: String,
    role: &'static str,
) -> AppResult<UserPublic> {
    let id = new_id();
    let now = now_iso();
    let hash = hash_password(&password)?;

    conn.execute(
        "INSERT INTO users (id, name, username, password_hash, role, active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
        params![id, name.trim(), username.trim(), hash, role, now],
    )?;

    let user = conn.query_row("SELECT * FROM users WHERE id = ?1", params![id], row_to_user)?;
    Ok(user)
}

pub fn list(conn: &mut Connection) -> AppResult<Vec<UserPublic>> {
    let mut stmt = conn.prepare("SELECT * FROM users ORDER BY created_at ASC")?;
    let rows = stmt.query_map([], row_to_user)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn set_active(conn: &mut Connection, user_id: String, active: bool) -> AppResult<()> {
    conn.execute(
        "UPDATE users SET active = ?1, updated_at = ?2 WHERE id = ?3",
        params![active as i64, now_iso(), user_id],
    )?;
    Ok(())
}

/// Implementación PostgreSQL (issue #9, Fase 2). `active` ya es `BOOLEAN`
/// nativo (sin el `!= 0` que hace falta en SQLite), y `resume_session` usa
/// `now()` del lado del servidor en vez de pasar la hora calculada en el
/// cliente — evita desfases de reloj entre la app y la base, con el mismo
/// efecto de "¿sigue vigente la sesión?". El resto es un espejo directo.
pub mod postgres {
    use sqlx::postgres::PgRow;
    use sqlx::{PgPool, Row};

    use crate::auth::{generate_session_token, hash_password, verify_password};
    use crate::db::{format_pg_timestamp, new_id};
    use crate::error::{AppError, AppResult};
    use crate::models::user::UserPublic;

    const SESSION_TTL_DAYS: i64 = 30;
    const USER_COLUMNS: &str = "id, name, username, role, active, created_at, updated_at";

    fn map_user(row: &PgRow) -> AppResult<UserPublic> {
        let created_at: time::OffsetDateTime = row.try_get("created_at")?;
        let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
        Ok(UserPublic {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            username: row.try_get("username")?,
            role: row.try_get("role")?,
            active: row.try_get("active")?,
            created_at: format_pg_timestamp(created_at),
            updated_at: format_pg_timestamp(updated_at),
        })
    }

    pub async fn login(
        pool: &PgPool,
        username: String,
        password: String,
        remember_me: bool,
    ) -> AppResult<(UserPublic, Option<String>)> {
        let username_norm = username.trim().to_lowercase();
        let row = sqlx::query(&format!(
            "SELECT {USER_COLUMNS}, password_hash FROM users
             WHERE lower(username) = $1 AND active = TRUE"
        ))
        .bind(username_norm)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::validation("Usuario o contraseña incorrectos"))?;

        let user = map_user(&row)?;
        let password_hash: String = row.try_get("password_hash")?;

        if !verify_password(&password, &password_hash) {
            return Err(AppError::validation("Usuario o contraseña incorrectos"));
        }

        let session_token = if remember_me {
            let token = generate_session_token();
            let expires_at =
                time::OffsetDateTime::now_utc().saturating_add(time::Duration::days(SESSION_TTL_DAYS));
            sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES ($1, $2, $3)")
                .bind(&token)
                .bind(&user.id)
                .bind(expires_at)
                .execute(pool)
                .await?;
            Some(token)
        } else {
            None
        };

        Ok((user, session_token))
    }

    pub async fn delete_session(pool: &PgPool, token: String) -> AppResult<()> {
        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(token)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Busca una sesión vigente y su usuario. Igual que la implementación
    /// SQLite: cualquier error de la consulta (no solo "sin filas") se trata
    /// como sesión inválida, no como un error a propagar.
    pub async fn resume_session(pool: &PgPool, session_token: String) -> AppResult<Option<UserPublic>> {
        let row: Option<PgRow> = sqlx::query(
            "SELECT u.id, u.name, u.username, u.role, u.active, u.created_at, u.updated_at
             FROM sessions s
             JOIN users u ON u.id = s.user_id
             WHERE s.token = $1 AND s.expires_at > now() AND u.active = TRUE",
        )
        .bind(session_token)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        row.as_ref().map(map_user).transpose()
    }

    pub async fn change_password(
        pool: &PgPool,
        user_id: String,
        old_password: Option<String>,
        new_password: String,
        verify_old_password: bool,
    ) -> AppResult<()> {
        if verify_old_password {
            let existing_hash: String =
                sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
                    .bind(&user_id)
                    .fetch_optional(pool)
                    .await?
                    .ok_or(AppError::NotFound)?;
            let old = old_password.unwrap_or_default();
            if !verify_password(&old, &existing_hash) {
                return Err(AppError::validation("La contraseña actual no es correcta"));
            }
        }

        if new_password.len() < 6 {
            return Err(AppError::validation(
                "La nueva contraseña debe tener al menos 6 caracteres",
            ));
        }

        let new_hash = hash_password(&new_password)?;
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = now() WHERE id = $2")
            .bind(new_hash)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn create_staff(
        pool: &PgPool,
        name: String,
        username: String,
        password: String,
        role: &'static str,
    ) -> AppResult<UserPublic> {
        let hash = hash_password(&password)?;

        let row = sqlx::query(&format!(
            "INSERT INTO users (id, name, username, password_hash, role, active)
             VALUES ($1, $2, $3, $4, $5, TRUE)
             RETURNING {USER_COLUMNS}"
        ))
        .bind(new_id())
        .bind(name.trim())
        .bind(username.trim())
        .bind(hash)
        .bind(role)
        .fetch_one(pool)
        .await?;

        map_user(&row)
    }

    pub async fn list(pool: &PgPool) -> AppResult<Vec<UserPublic>> {
        let rows = sqlx::query(&format!(
            "SELECT {USER_COLUMNS} FROM users ORDER BY created_at ASC"
        ))
        .fetch_all(pool)
        .await?;

        rows.iter().map(map_user).collect()
    }

    pub async fn set_active(pool: &PgPool, user_id: String, active: bool) -> AppResult<()> {
        sqlx::query("UPDATE users SET active = $1, updated_at = now() WHERE id = $2")
            .bind(active)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
