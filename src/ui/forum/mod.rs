use std::sync::Arc;

use sea_orm::DatabaseConnection;
use ssh_ui::{
    cursive::{
        event::{AnyCb, Event, EventResult},
        view::Selector,
        views::{EditView, LinearLayout, ResizedView, TextView},
        Printer, Vec2, View,
    },
    russh_keys::key::PublicKey,
};
use tokio::{
    spawn,
    sync::{
        mpsc::{channel, Sender},
        Mutex,
    },
};

use super::{get_user, labeled_edit_view::LabeledEditView};

lazy_static! {
    static ref CHAT_SENDERS: Mutex<Vec<Sender<String>>> = Mutex::new(Vec::new());
}

pub struct ChatBoxView {
    inner: ResizedView<LinearLayout>,
    messages: Arc<Mutex<Vec<String>>>,
    _handle: tokio::task::JoinHandle<()>,
}

impl ChatBoxView {
    fn get_text_view(&mut self) -> &mut TextView {
        self.inner
            .get_inner_mut()
            .get_child_mut(0)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<ResizedView<TextView>>()
            .unwrap()
            .get_inner_mut()
    }

    pub fn new(
        db: Arc<Mutex<DatabaseConnection>>,
        user: Option<PublicKey>,
        relayout_sender: Sender<()>,
    ) -> Self {
        let mut inner = LinearLayout::vertical();
        let user = get_user(db.clone(), user).unwrap();
        let user_cloned = user.clone();
        inner.add_child(ResizedView::with_full_screen(TextView::new("Chat box")));
        inner.add_child(LabeledEditView::new(
            "Message: ",
            None,
            "",
            |_, _, _| {},
            move |siv, message| {
                let message = format!("{}: {}", user_cloned.clone().handle, message);
                let mut senders = CHAT_SENDERS.blocking_lock();
                *senders = senders
                    .clone()
                    .into_iter()
                    .filter(|sender| sender.blocking_send(message.to_string()).is_ok())
                    .collect();
                siv.find_name::<EditView>("chat_edit_box")
                    .unwrap()
                    .set_content("");
            },
            "chat_edit_box",
        ));
        let (message_sender, mut message_receiver) = channel(5);
        let mut senders = CHAT_SENDERS.blocking_lock();
        senders.push(message_sender);
        inner.set_focus_index(1).unwrap();
        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_cloned = messages.clone();
        let handle = spawn(async move {
            loop {
                match message_receiver.recv().await {
                    Some(message) => {
                        messages_cloned.lock().await.push(message);
                        relayout_sender.send(()).await.unwrap();
                    }
                    None => break,
                }
            }
        });

        senders.iter().for_each(|sender| {
            let _ = sender.blocking_send(format!("<join> {}", user.handle));
        });
        Self {
            inner: ResizedView::with_full_screen(inner),
            messages,
            _handle: handle,
        }
    }
}

impl View for ChatBoxView {
    fn draw(&self, printer: &Printer) {
        self.inner.draw(printer)
    }
    fn needs_relayout(&self) -> bool {
        true
    }
    fn on_event(&mut self, event: Event) -> EventResult {
        self.inner.on_event(event)
    }
    fn call_on_any(&mut self, selector: &Selector, cb: AnyCb) {
        self.inner.call_on_any(selector, cb)
    }
    fn type_name(&self) -> &'static str {
        "ChatBoxView"
    }
    fn layout(&mut self, size: Vec2) {
        let text = self.messages.blocking_lock().join("\n");
        self.get_text_view().set_content(text);
        self.inner.layout(size)
    }
}
