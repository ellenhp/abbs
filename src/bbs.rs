use std::sync::mpsc::Sender;

use ssh_ui::{
    cursive::{
        view::{Nameable, Resizable},
        Cursive,
    },
    russh_keys::key::PublicKey,
    App, AppSession, SessionHandle,
};

use crate::ui::{
    library::search::LibrarySearchView,
    stack::{Stack, STACK_NAME},
};

pub(crate) struct BbsApp {}

impl App for BbsApp {
    fn on_load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // No-op
        Ok(())
    }

    fn new_session(&self) -> Box<dyn ssh_ui::AppSession> {
        Box::new(BbsAppSession {
            relayout_sender: None,
        })
    }
}

struct BbsAppSession {
    relayout_sender: Option<Sender<()>>,
}

impl BbsAppSession {}

impl AppSession for BbsAppSession {
    fn on_start(
        &mut self,
        _siv: &mut Cursive,
        _handle: SessionHandle,
        _pub_key: PublicKey,
        force_relayout_sender: Sender<()>,
    ) -> Result<Box<dyn ssh_ui::cursive::View>, Box<dyn std::error::Error>> {
        let mut stack = Stack::new(force_relayout_sender.clone());
        stack
            .push(Box::new(LibrarySearchView::new(
                "library",
                force_relayout_sender.clone(),
            )))
            .unwrap();
        self.relayout_sender = Some(force_relayout_sender);
        Ok(Box::new(stack.with_name(STACK_NAME).full_screen()))
    }

    fn on_tick(
        &mut self,
        _siv: &mut ssh_ui::cursive::Cursive,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
