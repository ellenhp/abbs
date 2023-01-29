use std::sync::{Arc, Mutex};

use sea_orm::DatabaseConnection;

pub mod gen;

pub struct DbUtil {
    db: Arc<Mutex<DatabaseConnection>>,
}

impl DbUtil {
    pub fn new(db: Arc<Mutex<DatabaseConnection>>) -> DbUtil {
        DbUtil { db }
    }
}
