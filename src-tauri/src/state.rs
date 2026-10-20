use std::sync::Mutex;

use rusqlite::Connection;

use crate::models::user::UserPublic;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub current_user: Mutex<Option<UserPublic>>,
}

impl AppState {
    pub fn new(db: Connection) -> Self {
        Self {
            db: Mutex::new(db),
            current_user: Mutex::new(None),
        }
    }
}
