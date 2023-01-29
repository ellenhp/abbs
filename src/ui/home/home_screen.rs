use std::{
    fmt::format,
    sync::{mpsc::Sender, Arc, Mutex},
};

use figlet_rs::FIGfont;
use sea_orm::DatabaseConnection;
use ssh_ui::{
    cursive::{
        direction::Orientation,
        views::{DummyView, LinearLayout, SelectView, TextView},
        View,
    },
    russh_keys::key::PublicKey,
};
use tokio::{runtime::Handle, task::block_in_place};

use crate::{
    ui::{library::search::LibrarySearchView, profile::profile_screen, stack::get_stack},
    user::UserUtil,
};

enum HomeOption {
    Profile,
    Forum,
    Library,
    Disconnect,
}

pub fn home_screen(
    force_relayout_sender: Sender<()>,
    db: Arc<Mutex<DatabaseConnection>>,
    key: Option<PublicKey>,
) -> Box<dyn View> {
    let mut select_view = SelectView::new()
        .item("Edit your (P)rofile", HomeOption::Profile)
        .item(
            "(UNIMPLEMENTED) (F)orum: Discussion boards for various topics",
            HomeOption::Forum,
        )
        .item("Visit the (L)ibrary", HomeOption::Library)
        .item("Disconnect", HomeOption::Disconnect);
    {
        let db = db.clone();
        let key = key.clone();
        select_view.set_on_submit(move |siv, item| match item {
            HomeOption::Profile => {
                get_stack(siv)
                    .push(profile_screen(db.clone(), key.clone()))
                    .unwrap();
            }
            HomeOption::Forum => {}
            HomeOption::Library => {
                get_stack(siv)
                    .push(Box::new(LibrarySearchView::new(
                        "library",
                        force_relayout_sender.clone(),
                    )))
                    .unwrap();
            }
            HomeOption::Disconnect => siv.quit(),
        });
    }
    let header = {
        let small_font = FIGfont::from_content(include_str!("./speed.flf")).unwrap();
        let figure = small_font.convert("BBS").unwrap();
        let user = block_in_place(move || {
            Handle::current().block_on(async move {
                let user_util = UserUtil::new(db.clone(), key.clone());
                user_util.get_user().await
            })
        });
        let welcome = if let Ok(user) = user {
            format!("Welcome to the Future, {}.", user.handle)
        } else {
            "Welcome to the Future".into()
        };
        TextView::new(format!("{}\n{}", figure, welcome))
    };
    let layout = LinearLayout::vertical()
        .child(header)
        .child(DummyView)
        .child(select_view);
    Box::new(layout)
}
