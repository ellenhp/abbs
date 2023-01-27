use std::sync::mpsc::Sender;

use ssh_ui::{
    cursive::{
        view::{Margins, Nameable, Resizable},
        views::Dialog,
        Cursive,
    },
    russh_keys::key::PublicKey,
    App, AppSession, SessionHandle,
};

use crate::ui::{
    home::home_screen::home_screen,
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
        _pub_key: Option<PublicKey>,
        force_relayout_sender: Sender<()>,
    ) -> Result<Box<dyn ssh_ui::cursive::View>, Box<dyn std::error::Error>> {
        let mut stack = Stack::new(force_relayout_sender.clone());
        stack
            .push(home_screen(force_relayout_sender.clone()))
            .unwrap();
        self.relayout_sender = Some(force_relayout_sender);
        let dialog = Dialog::new()
            .padding(Margins::lrtb(2, 2, 1, 1))
            .content(stack.with_name(STACK_NAME).full_screen())
            .full_screen();
        Ok(Box::new(dialog))
    }

    fn on_tick(
        &mut self,
        _siv: &mut ssh_ui::cursive::Cursive,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
