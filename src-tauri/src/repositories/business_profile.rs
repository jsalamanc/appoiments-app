use rusqlite::{params, Connection, Row};

use crate::auth::hash_password;
use crate::commands::business_profile::{OnboardingInput, OnboardingResult};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::business_profile::{BusinessProfile, ModulesFlags};
use crate::models::user::UserPublic;

fn row_to_profile(row: &Row) -> rusqlite::Result<BusinessProfile> {
    Ok(BusinessProfile {
        name: row.get("name")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_modules(row: &Row) -> rusqlite::Result<ModulesFlags> {
    Ok(ModulesFlags {
        employees: row.get::<_, i64>("module_employees")? != 0,
        appointments: row.get::<_, i64>("module_appointments")? != 0,
        services: row.get::<_, i64>("module_services")? != 0,
        packages: row.get::<_, i64>("module_packages")? != 0,
        reports: row.get::<_, i64>("module_reports")? != 0,
    })
}

pub fn onboarding_is_complete(conn: &mut Connection) -> AppResult<bool> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM business_profile", [], |r| r.get(0))?;
    Ok(count > 0)
}

pub fn onboarding_complete_setup(
    conn: &mut Connection,
    input: OnboardingInput,
) -> AppResult<OnboardingResult> {
    if input.business_name.trim().is_empty() {
        return Err(AppError::validation("El nombre del negocio es obligatorio"));
    }
    if input.owner_name.trim().is_empty() || input.owner_username.trim().is_empty() {
        return Err(AppError::validation("Nombre y usuario del dueño son obligatorios"));
    }
    if input.owner_password.len() < 6 {
        return Err(AppError::validation(
            "La contraseña debe tener al menos 6 caracteres",
        ));
    }

    let existing: i64 = conn.query_row("SELECT COUNT(*) FROM business_profile", [], |r| r.get(0))?;
    if existing > 0 {
        return Err(AppError::conflict("El negocio ya fue configurado"));
    }

    let tx = conn.transaction()?;
    let now = now_iso();

    tx.execute(
        "INSERT INTO business_profile (id, name, created_at, updated_at) VALUES (1, ?1, ?2, ?2)",
        params![input.business_name.trim(), now],
    )?;

    let owner_id = new_id();
    let hash = hash_password(&input.owner_password)?;
    tx.execute(
        "INSERT INTO users (id, name, username, password_hash, role, active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'owner', 1, ?5, ?5)",
        params![
            owner_id,
            input.owner_name.trim(),
            input.owner_username.trim(),
            hash,
            now
        ],
    )?;

    let profile = tx.query_row(
        "SELECT * FROM business_profile WHERE id = 1",
        [],
        row_to_profile,
    )?;
    let owner = tx.query_row(
        "SELECT * FROM users WHERE id = ?1",
        params![owner_id],
        |row| {
            Ok(UserPublic {
                id: row.get("id")?,
                name: row.get("name")?,
                username: row.get("username")?,
                role: row.get("role")?,
                active: row.get::<_, i64>("active")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    )?;

    tx.commit()?;

    Ok(OnboardingResult { profile, owner })
}

pub fn get_profile(conn: &mut Connection) -> AppResult<BusinessProfile> {
    let profile = conn.query_row("SELECT * FROM business_profile WHERE id = 1", [], row_to_profile)?;
    Ok(profile)
}

pub fn update_profile(conn: &mut Connection, name: String) -> AppResult<BusinessProfile> {
    if name.trim().is_empty() {
        return Err(AppError::validation("El nombre del negocio es obligatorio"));
    }
    conn.execute(
        "UPDATE business_profile SET name = ?1, updated_at = ?2 WHERE id = 1",
        params![name.trim(), now_iso()],
    )?;
    let profile = conn.query_row("SELECT * FROM business_profile WHERE id = 1", [], row_to_profile)?;
    Ok(profile)
}

pub fn get_modules(conn: &mut Connection) -> AppResult<ModulesFlags> {
    let modules = conn.query_row(
        "SELECT * FROM business_profile WHERE id = 1",
        [],
        row_to_modules,
    )?;
    Ok(modules)
}

pub fn update_modules(conn: &mut Connection, modules: ModulesFlags) -> AppResult<ModulesFlags> {
    let normalized = modules.normalize();
    conn.execute(
        "UPDATE business_profile SET
            module_employees = ?1,
            module_appointments = ?2,
            module_services = ?3,
            module_packages = ?4,
            module_reports = ?5,
            updated_at = ?6
         WHERE id = 1",
        params![
            normalized.employees as i64,
            normalized.appointments as i64,
            normalized.services as i64,
            normalized.packages as i64,
            normalized.reports as i64,
            now_iso(),
        ],
    )?;
    Ok(normalized)
}

/// Implementación PostgreSQL (issue #9, Fase 2). Los flags de módulo ya son
/// `BOOLEAN` nativo (sin el `!= 0`/`as i64` que hace falta en SQLite); el
/// resto es un espejo directo, incluida la transacción de
/// `onboarding_complete_setup` vía `pool.begin()`.
pub mod postgres {
    use sqlx::postgres::PgRow;
    use sqlx::{PgPool, Row};

    use crate::auth::hash_password;
    use crate::commands::business_profile::{OnboardingInput, OnboardingResult};
    use crate::db::{format_pg_timestamp, new_id};
    use crate::error::{AppError, AppResult};
    use crate::models::business_profile::{BusinessProfile, ModulesFlags};
    use crate::models::user::UserPublic;

    fn map_profile(row: &PgRow) -> AppResult<BusinessProfile> {
        let created_at: time::OffsetDateTime = row.try_get("created_at")?;
        let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
        Ok(BusinessProfile {
            name: row.try_get("name")?,
            created_at: format_pg_timestamp(created_at),
            updated_at: format_pg_timestamp(updated_at),
        })
    }

    fn map_modules(row: &PgRow) -> AppResult<ModulesFlags> {
        Ok(ModulesFlags {
            employees: row.try_get("module_employees")?,
            appointments: row.try_get("module_appointments")?,
            services: row.try_get("module_services")?,
            packages: row.try_get("module_packages")?,
            reports: row.try_get("module_reports")?,
        })
    }

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

    pub async fn onboarding_is_complete(pool: &PgPool) -> AppResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM business_profile")
            .fetch_one(pool)
            .await?;
        Ok(count > 0)
    }

    pub async fn onboarding_complete_setup(
        pool: &PgPool,
        input: OnboardingInput,
    ) -> AppResult<OnboardingResult> {
        if input.business_name.trim().is_empty() {
            return Err(AppError::validation("El nombre del negocio es obligatorio"));
        }
        if input.owner_name.trim().is_empty() || input.owner_username.trim().is_empty() {
            return Err(AppError::validation(
                "Nombre y usuario del dueño son obligatorios",
            ));
        }
        if input.owner_password.len() < 6 {
            return Err(AppError::validation(
                "La contraseña debe tener al menos 6 caracteres",
            ));
        }

        let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM business_profile")
            .fetch_one(pool)
            .await?;
        if existing > 0 {
            return Err(AppError::conflict("El negocio ya fue configurado"));
        }

        let mut tx = pool.begin().await?;

        sqlx::query("INSERT INTO business_profile (id, name) VALUES (1, $1)")
            .bind(input.business_name.trim())
            .execute(&mut *tx)
            .await?;

        let owner_id = new_id();
        let hash = hash_password(&input.owner_password)?;
        sqlx::query(
            "INSERT INTO users (id, name, username, password_hash, role, active)
             VALUES ($1, $2, $3, $4, 'owner', TRUE)",
        )
        .bind(&owner_id)
        .bind(input.owner_name.trim())
        .bind(input.owner_username.trim())
        .bind(hash)
        .execute(&mut *tx)
        .await?;

        let profile_row = sqlx::query("SELECT name, created_at, updated_at FROM business_profile WHERE id = 1")
            .fetch_one(&mut *tx)
            .await?;
        let profile = map_profile(&profile_row)?;

        let owner_row = sqlx::query(
            "SELECT id, name, username, role, active, created_at, updated_at
             FROM users WHERE id = $1",
        )
        .bind(&owner_id)
        .fetch_one(&mut *tx)
        .await?;
        let owner = map_user(&owner_row)?;

        tx.commit().await?;

        Ok(OnboardingResult { profile, owner })
    }

    pub async fn get_profile(pool: &PgPool) -> AppResult<BusinessProfile> {
        let row = sqlx::query("SELECT name, created_at, updated_at FROM business_profile WHERE id = 1")
            .fetch_one(pool)
            .await?;
        map_profile(&row)
    }

    pub async fn update_profile(pool: &PgPool, name: String) -> AppResult<BusinessProfile> {
        if name.trim().is_empty() {
            return Err(AppError::validation("El nombre del negocio es obligatorio"));
        }
        let row = sqlx::query(
            "UPDATE business_profile SET name = $1, updated_at = now() WHERE id = 1
             RETURNING name, created_at, updated_at",
        )
        .bind(name.trim())
        .fetch_one(pool)
        .await?;
        map_profile(&row)
    }

    pub async fn get_modules(pool: &PgPool) -> AppResult<ModulesFlags> {
        let row = sqlx::query(
            "SELECT module_employees, module_appointments, module_services, module_packages, module_reports
             FROM business_profile WHERE id = 1",
        )
        .fetch_one(pool)
        .await?;
        map_modules(&row)
    }

    pub async fn update_modules(pool: &PgPool, modules: ModulesFlags) -> AppResult<ModulesFlags> {
        let normalized = modules.normalize();
        sqlx::query(
            "UPDATE business_profile SET
                module_employees = $1,
                module_appointments = $2,
                module_services = $3,
                module_packages = $4,
                module_reports = $5,
                updated_at = now()
             WHERE id = 1",
        )
        .bind(normalized.employees)
        .bind(normalized.appointments)
        .bind(normalized.services)
        .bind(normalized.packages)
        .bind(normalized.reports)
        .execute(pool)
        .await?;
        Ok(normalized)
    }
}
