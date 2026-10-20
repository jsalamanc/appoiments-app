use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sale {
    pub id: String,
    pub customer_id: String,
    pub appointment_id: Option<String>,
    pub payment_method: String,
    pub total: f64,
    pub sold_at: String,
    pub created_at: String,
}

impl Sale {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            customer_id: row.get("customer_id")?,
            appointment_id: row.get("appointment_id")?,
            payment_method: row.get("payment_method")?,
            total: row.get("total")?,
            sold_at: row.get("sold_at")?,
            created_at: row.get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItem {
    pub id: String,
    pub sale_id: String,
    pub item_type: String,
    pub product_id: Option<String>,
    pub service_id: Option<String>,
    pub package_id: Option<String>,
    pub name_snapshot: String,
    pub unit_price: f64,
    pub quantity: i64,
    pub line_total: f64,
    pub created_at: String,
}

impl SaleItem {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            sale_id: row.get("sale_id")?,
            item_type: row.get("item_type")?,
            product_id: row.get("product_id")?,
            service_id: row.get("service_id")?,
            package_id: row.get("package_id")?,
            name_snapshot: row.get("name_snapshot")?,
            unit_price: row.get("unit_price")?,
            quantity: row.get("quantity")?,
            line_total: row.get("line_total")?,
            created_at: row.get("created_at")?,
        })
    }
}

/// Vista mínima de un cliente embebida en `SaleWithDetails`.
/// Definida aquí (en vez de reutilizar la del dominio `customers`) para no
/// tocar `commands/customers.rs`, que pertenece a otro dominio ya completado.
/// Mismos nombres de campo (snake_case) que `commands::customers::CustomerRef`
/// y que `Pick<Customer, "id"|"name"|"email"|"phone"|"document">` en
/// `src/types/database.ts`.
#[derive(Debug, Clone, Serialize)]
pub struct CustomerRef {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub document: String,
}

/// Debe coincidir exactamente con `SaleWithDetails` de `src/types/database.ts`
/// (`Sale & { customers: ..., sale_items: SaleItem[] }`).
#[derive(Debug, Serialize)]
pub struct SaleWithDetails {
    pub id: String,
    pub customer_id: String,
    pub appointment_id: Option<String>,
    pub payment_method: String,
    pub total: f64,
    pub sold_at: String,
    pub created_at: String,
    pub customers: CustomerRef,
    pub sale_items: Vec<SaleItem>,
}
