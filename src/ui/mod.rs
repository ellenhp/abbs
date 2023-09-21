use std::sync::Arc;

use sea_orm::DatabaseConnection;
use ssh_ui::russh_keys::key::PublicKey;
use tokio::{runtime::Handle, sync::Mutex, task::block_in_place};

use crate::user::{UserInfo, UserUtil};

pub(crate) mod forum;
pub(crate) mod home;
pub(crate) mod labeled_edit_view;
pub(crate) mod library;
pub(crate) mod profile;
pub(crate) mod stack;

pub fn get_user(
    db: Arc<Mutex<DatabaseConnection>>,
    key: Option<PublicKey>,
) -> Result<UserInfo, anyhow::Error> {
    block_in_place(move || {
        Handle::current().block_on(async move {
            let user_util = UserUtil::new(db.clone(), key.clone());
            user_util.get_user().await
        })
    })
}
