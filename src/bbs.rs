use std::sync::{mpsc::Sender, Arc, Mutex};

use log::info;
use sea_orm::DatabaseConnection;
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

pub(crate) struct BbsApp {
    db: Arc<Mutex<DatabaseConnection>>,
}

impl App for BbsApp {
    fn on_load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // No-op
        Ok(())
    }

    fn new_session(&self) -> Box<dyn ssh_ui::AppSession> {
        Box::new(BbsAppSession {
            relayout_sender: None,
            db: self.db.clone(),
        })
    }
}

impl BbsApp {
    pub fn new_with_db(db: DatabaseConnection) -> BbsApp {
        BbsApp {
            db: Arc::new(Mutex::new(db)),
        }
    }
}

struct BbsAppSession {
    relayout_sender: Option<Sender<()>>,
    db: Arc<Mutex<DatabaseConnection>>,
}

impl BbsAppSession {}

impl AppSession for BbsAppSession {
    fn on_start(
        &mut self,
        siv: &mut Cursive,
        _handle: SessionHandle,
        pub_key: Option<PublicKey>,
        force_relayout_sender: Sender<()>,
    ) -> Result<Box<dyn ssh_ui::cursive::View>, Box<dyn std::error::Error>> {
        info!("Starting new session, user: {:?}", pub_key);
        let mut stack = Stack::new(siv, force_relayout_sender.clone(), self.db.clone());
        stack
            .push(home_screen(
                force_relayout_sender.clone(),
                self.db.clone(),
                pub_key.clone(),
            ))
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
