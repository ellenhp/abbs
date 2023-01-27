use std::sync::{mpsc::Sender, Arc, Mutex};

use ssh_ui::cursive::{
    direction::Direction,
    event::{AnyCb, Event, EventResult},
    view::{CannotFocus, Nameable, Resizable, Selector, ViewNotFound},
    views::{EditView, LinearLayout, NamedView, ResizedView, SelectView},
    Printer, Vec2, View,
};
use tokio::spawn;

use crate::ui::stack::get_stack;

use super::{get_library, viewer::new_viewer, Article, Library};

fn search(lib: &Library, text: &str, limit: usize) -> Result<Vec<Article>, anyhow::Error> {
    let articles = lib.search(text, limit)?;
    Ok(articles)
}

async fn search_cb(
    text: &str,
    max_results: usize,
    lib_name: &str,
    search_result_repository: Arc<Mutex<(u64, u64, Arc<Vec<Article>>, bool)>>,
) -> Result<(), anyhow::Error> {
    if let Some(lib) = get_library(lib_name) {
        let text = text.to_string();
        let current_counter_at_start = {
            let mut result_tuple = search_result_repository.lock().unwrap();
            result_tuple.0 += 1;
            result_tuple.3 = true;
            result_tuple.0
        };

        if text.is_empty() {
            let mut result_tuple = search_result_repository.lock().unwrap();
            result_tuple.1 = current_counter_at_start;
            result_tuple.2 = Vec::new().into();
            result_tuple.3 = true;
            return Ok(());
        }

        let articles = search(&lib, &text, max_results);
        match articles {
            Ok(articles) => {
                let mut result_tuple = search_result_repository.lock().unwrap();
                if result_tuple.1 >= current_counter_at_start {
                    return Ok(());
                }
                result_tuple.1 = current_counter_at_start;
                result_tuple.2 = articles.into();
                result_tuple.3 = true;
            }
            Err(err) => {
                println!("Error during search: {}", err);
            }
        }
    }
    Ok(())
}

fn update_search_results(
    sv: &mut SelectView<Article>,
    search_result_repository: Arc<Mutex<(u64, u64, Arc<Vec<Article>>, bool)>>,
) {
    let mut result_tuple = search_result_repository.lock().unwrap();
    if !result_tuple.3 {
        return;
    }
    sv.clear();
    for (idx, article) in result_tuple.2.iter().enumerate() {
        sv.add_item(
            &format!("{:3}: {}", idx + 1, article.title),
            article.clone(),
        );
    }
    result_tuple.3 = false;
}

pub struct LibrarySearchView {
    inner: ResizedView<LinearLayout>,
    search_result_repository: Arc<Mutex<(u64, u64, Arc<Vec<Article>>, bool)>>,
}

impl LibrarySearchView {
    pub fn new(lib_name: &str, relayout_sender: Sender<()>) -> LibrarySearchView {
        let lib_name = lib_name.to_string();
        let search_result_repository = Arc::new(Mutex::new((0, 0, Arc::new(Vec::new()), false)));
        let search_box = {
            let search_result_repository = search_result_repository.clone();
            EditView::new()
                .filler("Search for a book")
                .on_edit_mut(move |siv, text, _cursor| {
                    let mut search_box = siv.find_name::<EditView>("library_search_box").unwrap();
                    if text.is_empty() {
                        search_box.set_filler("Search for a book");
                    } else {
                        // Can't set to empty string due to panic on division by zero in cursive.
                        search_box.set_filler(" ");
                    }

                    let max_results = siv.screen_size().y; // Upper bound.
                    let text = text.to_string();
                    let lib_name = lib_name.clone();
                    let search_result_repository = search_result_repository.clone();
                    let relayout_sender = relayout_sender.clone();
                    spawn(async move {
                        match search_cb(&text, max_results, &lib_name, search_result_repository)
                            .await
                        {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error during search: {}", err);
                            }
                        }
                        relayout_sender.send(()).unwrap();
                    });
                })
                .on_submit(|siv, search_term| {
                    if !search_term.is_empty() {
                        siv.focus_name("library_search_results").unwrap();
                    }
                })
        };
        let results_box = SelectView::<Article>::new().on_submit(|siv, item| {
            let viewer = new_viewer(siv, item.content_html.clone());
            get_stack(siv).push(viewer).unwrap();
        });
        LibrarySearchView {
            inner: LinearLayout::vertical()
                .child(search_box.with_name("library_search_box"))
                .child(results_box.with_name("library_search_results"))
                .full_screen(),
            search_result_repository,
        }
    }
}

impl View for LibrarySearchView {
    fn draw(&self, printer: &Printer) {
        self.inner.draw(printer)
    }

    fn layout(&mut self, size: Vec2) {
        self.inner.layout(size);
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.inner.required_size(constraint)
    }

    fn needs_relayout(&self) -> bool {
        true
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if event == Event::Refresh {
            let articles = self
                .inner
                .get_inner_mut()
                .get_child_mut(1)
                .unwrap()
                .as_any_mut()
                .downcast_mut::<NamedView<SelectView<Article>>>()
                .unwrap();
            update_search_results(
                &mut articles.get_mut(),
                self.search_result_repository.clone(),
            );
        }
        self.inner.on_event(event)
    }

    fn call_on_any(&mut self, selector: &Selector, cb: AnyCb) {
        if let Selector::Name(name) = selector {
            dbg!(name);
        }
        self.inner.call_on_any(selector, cb);
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.inner.focus_view(selector)
    }

    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        self.inner.take_focus(source)
    }

    fn important_area(&self, view_size: Vec2) -> ssh_ui::cursive::Rect {
        self.inner.important_area(view_size)
    }

    fn type_name(&self) -> &'static str {
        "LibrarySearchView"
    }
}
