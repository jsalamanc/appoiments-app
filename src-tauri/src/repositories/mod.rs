//! Capa de acceso a datos, un módulo por dominio, extraída de `commands/*.rs`
//! (issue #9, Fase 1). Cada función toma la conexión SQLite ya desbloqueada
//! (vía `AppDb::with_sqlite` en el comando que la invoca) y contiene
//! exactamente el mismo SQL que antes vivía inline en el comando.
//!
//! Los comandos de Tauri en `commands/*.rs` quedan como wrappers finos
//! `async fn` que delegan aquí; los tipos de entrada/salida (DTOs
//! serializables hacia el frontend) se quedan definidos en `commands/*.rs`,
//! ya que son parte de la superficie de la API de Tauri, no del acceso a
//! datos en sí.

pub mod appointments;
pub mod auth;
pub mod business_hours;
pub mod business_profile;
pub mod customers;
pub mod employees;
pub mod inventory;
pub mod packages;
pub mod products;
pub mod reports;
pub mod sales;
pub mod services;
