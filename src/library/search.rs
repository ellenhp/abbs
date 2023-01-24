use std::sync::{Arc, Mutex};

use ssh_ui::cursive::{
    event::Callback,
    view::{IntoBoxedView, Nameable, Resizable},
    views::{EditView, LinearLayout, SelectView},
    Cursive, View,
};
use tokio::spawn;

use super::{get_library, viewer::new_viewer, Article, Library};

fn search(lib: &Library, text: &str, limit: usize) -> Result<Vec<Article>, anyhow::Error> {
    let articles = lib.search(text, limit)?;
    Ok(articles)
}

fn search_cb(
    siv: &mut Cursive,
    text: &str,
    lib_name: &str,
    search_result_repository: Arc<Mutex<(u64, u64, Arc<Vec<Article>>, bool)>>,
) -> Result<(), anyhow::Error> {
    let mut search_box = siv.find_name::<EditView>("library_search_box").unwrap();
    if text.is_empty() {
        search_box.set_filler("Search for a book");

        let mut result_tuple = search_result_repository.lock().unwrap();
        result_tuple.0 += 1;
        result_tuple.1 = result_tuple.0;
        result_tuple.2 = Vec::new().into();

        return Ok(());
    } else {
        // Can't set to empty string due to panic on division by zero in cursive.
        search_box.set_filler(" ");
    }

    let max_results = siv.screen_size().y; // Upper bound.
    if let Some(lib) = get_library(lib_name) {
        let text = text.to_string();
        let current_counter_at_start = {
            let mut result_tuple = search_result_repository.lock().unwrap();
            result_tuple.0 += 1;
            result_tuple.3 = true;
            result_tuple.0
        };
        spawn(async move {
            let articles = search(&lib, &text, max_results);
            match articles {
                Ok(articles) => {
                    let mut result_tuple = search_result_repository.lock().unwrap();
                    if result_tuple.1 > current_counter_at_start {
                        return;
                    }
                    result_tuple.1 = current_counter_at_start;
                    result_tuple.2 = articles.into();
                    result_tuple.3 = true;
                }
                Err(err) => {
                    println!("Error during search: {}", err);
                }
            }
        });
    }
    Ok(())
}

fn update_search_results(
    siv: &mut Cursive,
    search_result_repository: Arc<Mutex<(u64, u64, Arc<Vec<Article>>, bool)>>,
) {
    let mut result_tuple = search_result_repository.lock().unwrap();
    if !result_tuple.3 {
        return;
    }
    let mut results_box = siv
        .find_name::<SelectView<Article>>("library_search_results")
        .unwrap();
    results_box.clear();
    for (idx, article) in result_tuple.2.iter().enumerate() {
        results_box.add_item(
            &format!("{:3}: {}", idx + 1, article.title),
            article.clone(),
        );
    }
    result_tuple.3 = false;
}

pub fn library_view(lib_name: &str) -> (Box<dyn View>, Callback) {
    let lib_name = lib_name.to_string();
    let search_result_repository = Arc::new(Mutex::new((0, 0, Arc::new(Vec::new()), false)));
    let search_box = {
        let search_result_repository = search_result_repository.clone();
        EditView::new()
            .filler("Search for a book")
            .on_edit_mut(move |s, text, _cursor| {
                match search_cb(s, text, &lib_name, search_result_repository.clone()) {
                    Ok(_) => {}
                    Err(err) => {
                        println!("Error during search: {}", err);
                    }
                }
            })
            .on_submit(|siv, search_term| {
                if !search_term.is_empty() {
                    siv.focus_name("library_search_results").unwrap();
                }
            })
            .with_name("library_search_box")
    };
    let results_box = SelectView::<Article>::new()
        .on_submit(|siv, item| {
            dbg!(item);
            let viewer = new_viewer(siv, item.content_html.clone());
            siv.add_layer(viewer);
        })
        .with_name("library_search_results");
    (
        LinearLayout::vertical()
            .child(search_box)
            .child(results_box)
            .full_screen()
            .into_boxed_view(),
        Callback::from_fn(move |siv| update_search_results(siv, search_result_repository.clone())),
    )
}
