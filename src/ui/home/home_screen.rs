use std::sync::mpsc::Sender;

use ssh_ui::cursive::{
    direction::Orientation,
    views::{DummyView, LinearLayout, SelectView, TextView},
    View,
};

use crate::ui::{library::search::LibrarySearchView, stack::get_stack};

enum HomeOption {
    Profile,
    Forum,
    Library,
    Disconnect,
}

pub fn home_screen(force_relayout_sender: Sender<()>) -> Box<dyn View> {
    let mut select_view = SelectView::new()
        .item("(UNIMPLEMENTED) Edit your (P)rofile", HomeOption::Profile)
        .item(
            "(UNIMPLEMENTED) (F)orum: Discussion boards for various topics",
            HomeOption::Forum,
        )
        .item("Visit the (L)ibrary", HomeOption::Library)
        .item("Disconnect", HomeOption::Disconnect);
    select_view.set_on_submit(move |siv, item| match item {
        HomeOption::Profile => {}
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
    let header = TextView::new("Welcome to the Future");
    let layout = LinearLayout::new(Orientation::Vertical)
        .child(header)
        .child(DummyView)
        .child(select_view);
    Box::new(layout)
}
