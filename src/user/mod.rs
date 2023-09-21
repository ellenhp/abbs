use std::sync::Arc;

use crate::db::gen::prelude::PublicKey;
use crate::db::gen::{public_key, user};
use anyhow::Ok;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    QueryFilter, Set,
};
use ssh_ui::russh_keys::key::PublicKey as RusshPublicKey;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct UserId(i32);

pub struct UserUtil {
    db: Arc<Mutex<DatabaseConnection>>,
    key: Option<RusshPublicKey>,
    _active_user: Arc<Mutex<Option<i32>>>,
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    _id: Option<UserId>,
    pub handle: String,
    pub contact: String,
}

impl Default for UserInfo {
    fn default() -> Self {
        Self {
            _id: None,
            handle: Default::default(),
            contact: Default::default(),
        }
    }
}

#[derive(Debug, Error)]
enum UserUtilError {
    #[error("Key not present")]
    KeyNotPresent,
    #[error("User not registered")]
    NotRegistered,
    #[error("Key is present but user is not")]
    DatabaseConsistencyError,
}

impl UserUtil {
    pub fn new(db: Arc<Mutex<DatabaseConnection>>, key: Option<RusshPublicKey>) -> UserUtil {
        UserUtil {
            db,
            key,
            _active_user: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn set_user(&self, handle: &str, contact: &str) -> Result<(), anyhow::Error> {
        let key = if let Some(key) = &self.key {
            key
        } else {
            return Err(UserUtilError::KeyNotPresent.into());
        };
        let mut db = self.db.lock().await.to_owned(); // TODO: Remove unwrap with anyhow

        if let Result::Ok(_) = self.get_user().await {
            // TODO: Do this atomically.
            let key = PublicKey::find()
                .filter(public_key::Column::Fingerprint.eq(key.fingerprint()))
                .one(&db)
                .await?;

            if let Some(key) = key {
                if let Some(user) = key.find_related(user::Entity).one(&db).await? {
                    let mut active = user.into_active_model();
                    active.handle = Set(handle.to_string());
                    active.contact = Set(Some(contact.to_string()));
                    active.update(&mut db).await?;
                } else {
                    return Err(UserUtilError::DatabaseConsistencyError.into());
                }
            } else {
                return Err(UserUtilError::NotRegistered.into());
            }
        } else {
            // TODO: Do this atomically.
            let user_model = user::ActiveModel {
                handle: Set(handle.to_string()),
                contact: Set(Some(contact.to_string())),
                ..Default::default()
            };
            let user_model = user_model.insert(&mut db).await?;

            let key_model = public_key::ActiveModel {
                fingerprint: Set(key.fingerprint()),
                user_id: Set(user_model.id),
                ..Default::default()
            };
            key_model.insert(&mut db).await?;
        }
        Ok(())
    }

    pub async fn get_user(&self) -> Result<UserInfo, anyhow::Error> {
        let key = if let Some(key) = &self.key {
            key
        } else {
            return Err(UserUtilError::KeyNotPresent.into());
        };
        let db = self.db.lock().await.to_owned(); // TODO: Remove unwrap with anyhow

        let key = PublicKey::find()
            .filter(public_key::Column::Fingerprint.eq(key.fingerprint()))
            .one(&db)
            .await?;

        if let Some(key) = key {
            if let Some(user) = key.find_related(user::Entity).one(&db).await? {
                Ok(UserInfo {
                    _id: Some(UserId(user.id)),
                    handle: user.handle,
                    contact: user.contact.unwrap_or("".into()),
                })
            } else {
                Err(UserUtilError::DatabaseConsistencyError.into())
            }
        } else {
            Err(UserUtilError::NotRegistered.into())
        }
    }
}
