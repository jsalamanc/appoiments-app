use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserRole {
    Owner,
    Staff,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Owner => "owner",
            UserRole::Staff => "staff",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "owner" => UserRole::Owner,
            _ => UserRole::Staff,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPublic {
    pub id: String,
    pub name: String,
    pub username: String,
    pub role: String,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}
