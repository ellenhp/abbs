use std::sync::Arc;

use sea_orm::DatabaseConnection;
use ssh_ui::cursive::direction::Direction;
use ssh_ui::cursive::event::EventResult::Ignored;
use ssh_ui::cursive::event::{AnyCb, Event, EventResult, Key};
use ssh_ui::cursive::view::{CannotFocus, Selector, ViewNotFound};
use ssh_ui::cursive::views::ViewRef;
use ssh_ui::cursive::{Cursive, Rect, Vec2, View};
use thiserror::Error;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum StackError {
    #[error("Error during force relayout")]
    ForceRelayoutError(#[from] tokio::sync::mpsc::error::SendError<()>),
    #[error("Stack empty, session over")]
    StackEmpty,
}

pub static STACK_NAME: &str = "STACK_NAME";

pub fn get_stack<'a>(siv: &'a mut Cursive) -> ViewRef<Stack> {
    siv.find_name::<Stack>(STACK_NAME).unwrap()
}

pub struct Stack {
    stack: Arc<Mutex<Vec<Box<dyn View>>>>,
    dirty: bool,
    relayout_sender: Sender<()>,
    _db: Arc<Mutex<DatabaseConnection>>,
}

impl Stack {
    pub fn new(
        siv: &mut Cursive,
        relayout_sender: Sender<()>,
        db: Arc<Mutex<DatabaseConnection>>,
    ) -> Self {
        let mut stack = Self {
            stack: Arc::new(Mutex::new(Vec::new())),
            dirty: true,
            relayout_sender,
            _db: db,
        };
        stack.setup_esc(siv);
        stack
    }

    pub fn push(&mut self, view: Box<dyn View>) -> Result<(), StackError> {
        self.dirty = true;
        self.stack.blocking_lock().push(view);
        self.relayout_sender
            .blocking_send(())
            .map_err(|err| StackError::ForceRelayoutError(err))?;
        Ok(())
    }

    pub fn pop(&mut self, siv: &mut Cursive) -> Result<Box<dyn View>, StackError> {
        let ret = self.stack.blocking_lock().pop();
        self.dirty = true;
        self.relayout_sender
            .blocking_send(())
            .map_err(|err| StackError::ForceRelayoutError(err))?;
        match ret {
            Some(old) => {
                if self.stack.blocking_lock().is_empty() {
                    siv.quit();
                }

                Ok(old)
            }
            None => {
                siv.quit();
                Err(StackError::StackEmpty)
            }
        }
    }

    fn setup_esc(&mut self, siv: &mut Cursive) {
        let stack_clone = self.stack.clone();
        siv.set_on_post_event(Event::Key(Key::Esc), move |siv| {
            let mut stack = stack_clone.blocking_lock();
            stack.pop();
            if stack.len() == 0 {
                siv.quit();
            }
        })
    }
}

impl View for Stack {
    fn layout(&mut self, size: Vec2) {
        // Layout every view in the stack just in case this is only called once or something weird like that.
        for view in self.stack.blocking_lock().iter_mut() {
            view.layout(size);
        }
        self.dirty = true;
    }

    fn needs_relayout(&self) -> bool {
        self.dirty
            || self
                .stack
                .blocking_lock()
                .last()
                .map(|view| view.needs_relayout())
                .unwrap_or(false)
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.stack
            .blocking_lock()
            .last_mut()
            .map(|view| view.required_size(constraint))
            .unwrap_or_default()
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        self.stack
            .blocking_lock()
            .last_mut()
            .map_or(Ignored, |view| view.on_event(event))
    }

    fn call_on_any(&mut self, selector: &Selector, cb: AnyCb) {
        match selector {
            Selector::Name(name) => {
                println!("{name}");
                self.stack
                    .blocking_lock()
                    .last_mut()
                    .map(|view| view.call_on_any(selector, cb));
            }
            _ => {
                self.stack
                    .blocking_lock()
                    .last_mut()
                    .map(|view| view.call_on_any(selector, cb));
            }
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.stack
            .blocking_lock()
            .last_mut()
            .map(|view| view.focus_view(selector))
            .unwrap_or(Err(ViewNotFound))
    }

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        self.stack
            .blocking_lock()
            .last_mut()
            .map(|view| view.take_focus(source))
            .unwrap_or(Err(CannotFocus))
    }

    fn important_area(&self, view_size: Vec2) -> ssh_ui::cursive::Rect {
        self.stack
            .blocking_lock()
            .last()
            .map(|view| view.important_area(view_size))
            .unwrap_or(Rect::from_point(Vec2::zero()))
    }

    fn type_name(&self) -> &'static str {
        "Stack"
    }

    fn draw(&self, printer: &ssh_ui::cursive::Printer) {
        self.stack
            .blocking_lock()
            .last()
            .map(|view| view.draw(printer))
            .unwrap_or_default();
    }
}
