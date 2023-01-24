use russh_keys::key::PublicKey;
use ssh_ui::{
    cursive::{
        event::{Callback, Event, Key},
        Cursive,
    },
    App, AppSession, SessionHandle,
};

use crate::library::search::library_view;

pub(crate) struct BbsApp {}

impl App for BbsApp {
    fn on_load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // No-op
        Ok(())
    }

    fn new_session(&self) -> Box<dyn ssh_ui::AppSession> {
        Box::new(BbsAppSession {
            callbacks: Vec::new(),
        })
    }
}

struct BbsAppSession {
    callbacks: Vec<Callback>,
}

impl BbsAppSession {}

impl AppSession for BbsAppSession {
    fn on_start(
        &mut self,
        siv: &mut Cursive,
        _handle: SessionHandle,
        _pub_key: PublicKey,
    ) -> Result<Box<dyn ssh_ui::cursive::View>, Box<dyn std::error::Error>> {
        let (library_view, cb) = library_view("library");
        self.callbacks.push(cb);
        siv.set_on_post_event(Event::Char('q'), move |siv| {
            siv.pop_layer();
            siv.focus_name("library_search_box").unwrap();
        });
        siv.set_on_post_event(Event::Key(Key::Esc), move |siv| {
            siv.pop_layer();
            siv.focus_name("library_search_box").unwrap();
        });

        Ok(library_view)
    }

    fn on_tick(
        &mut self,
        siv: &mut ssh_ui::cursive::Cursive,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.callbacks.iter().for_each(|cb| (cb)(siv));
        Ok(())
    }
}
