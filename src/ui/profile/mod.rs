use std::sync::Arc;

use sea_orm::DatabaseConnection;
use ssh_ui::{
    cursive::{
        direction::Orientation,
        view::Resizable,
        views::{LinearLayout, TextView},
        View,
    },
    russh_keys::key::PublicKey,
};
use tokio::{runtime::Handle, sync::Mutex, task::block_in_place};

use crate::user::UserUtil;

use super::{labeled_edit_view::LabeledEditView, stack::get_stack};

pub fn profile_screen(db: Arc<Mutex<DatabaseConnection>>, key: Option<PublicKey>) -> Box<dyn View> {
    let user = {
        let db = db.clone();
        let key = key.clone();
        block_in_place(move || {
            Handle::current().block_on(async move {
                let user_util = UserUtil::new(db.clone(), key.clone());
                user_util.get_user().await
            })
        })
    };
    if let Some(key) = key {
        let blurb = TextView::new("Enter profile information to access forums. Contact information (Matrix handle, etc) is optional.");

        let (initial_handle, initial_contact) = if let Ok(user) = user {
            (user.handle, user.contact)
        } else {
            ("".into(), "".into())
        };
        let min_width = 10;
        let handle_label = "profile-handle-edit";
        let contact_label = "profile-contact-edit";

        let handle_val = Arc::new(Mutex::new(initial_handle.clone()));
        let contact_val = Arc::new(Mutex::new(initial_contact.clone()));
        let handle = {
            let handle_val = handle_val.clone();
            LabeledEditView::new(
                "Handle:",
                Some(min_width),
                &initial_handle,
                move |_siv, val, _cursor| {
                    *handle_val.blocking_lock() = val.to_string();
                },
                |siv, _val| {
                    siv.focus_name(contact_label).unwrap();
                },
                handle_label,
            )
        };
        let contact = {
            let contact_val = contact_val.clone();
            let db = db.clone();
            let key = key.clone();
            let handle_val = handle_val.clone();
            let contact_val = contact_val.clone();
            let contact_val_submit = contact_val.clone();
            LabeledEditView::new(
                "Contact:",
                Some(min_width),
                &initial_contact,
                move |_siv, val, _cursor| {
                    *contact_val.blocking_lock() = val.to_string();
                },
                move |siv, _val| {
                    let db = db.clone();
                    let key = key.clone();
                    let handle_val = handle_val.clone();
                    let contact_val = contact_val_submit.clone();
                    block_in_place(move || {
                        Handle::current().block_on(async move {
                            let user_util = UserUtil::new(db.clone(), Some(key.clone()));
                            user_util
                                .set_user(
                                    &*handle_val.lock().await,
                                    &*contact_val.lock().await,
                                )
                                .await
                                .unwrap();
                        });
                    });
                    get_stack(siv).pop(siv).unwrap();
                },
                contact_label,
            )
        };

        let mut layout = LinearLayout::new(Orientation::Vertical)
            .child(blurb)
            .child(handle.full_width())
            .child(contact.full_width());
        layout.set_focus_index(1).unwrap();

        Box::new(layout.full_screen())
    } else {
        let blurb = TextView::new("Enter profile information to access forums. Contact information (Matrix handle, etc) is optional.\n\nAnonymous users can't set profile information. Log in with an ssh public key to continue.");
        Box::new(blurb)
    }
}
