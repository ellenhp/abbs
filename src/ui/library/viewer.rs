use regex::Regex;
use ssh_ui::cursive::{
    direction::{Direction, Orientation},
    event::{AnyCb, Callback, Event, EventResult, Key},
    reexports::enumset,
    theme::{Color, ColorStyle, Effect, Style},
    utils::span::{IndexedCow, IndexedSpan, SpannedString},
    view::{scroll::Scroller, CannotFocus, Resizable, Scrollable, Selector, ViewNotFound},
    views::{EditView, LinearLayout, ResizedView, ScrollView, TextView},
    Printer, Rect, Vec2, View,
};

use crate::ui::stack::get_stack;

pub struct ReaderView {
    inner: ResizedView<LinearLayout>,
    size: Vec2,
    html: String,
    text_wrapped: String,
    line_offsets: Vec<usize>,
    current_match: Option<(usize, usize)>,
}

impl ReaderView {
    pub fn new(html: &str) -> ReaderView {
        let reader = TextView::new("Loading...").scrollable().full_screen();
        let searcher = EditView::new().disabled().full_width();
        ReaderView {
            inner: LinearLayout::new(Orientation::Vertical)
                .child(reader)
                .child(searcher)
                .full_screen(),
            size: Vec2::new(1, 1),
            html: html.to_string(),
            text_wrapped: "".into(),
            line_offsets: vec![],
            current_match: None,
        }
    }

    fn get_reader<'a>(&'a mut self) -> &'a mut ScrollView<TextView> {
        self.inner
            .get_inner_mut()
            .get_child_mut(0)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<ResizedView<ScrollView<TextView>>>()
            .unwrap()
            .get_inner_mut()
    }

    fn get_search<'a>(&'a mut self) -> &'a mut EditView {
        self.inner
            .get_inner_mut()
            .get_child_mut(1)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<ResizedView<EditView>>()
            .unwrap()
            .get_inner_mut()
    }

    fn find_line(&self, char: usize) -> usize {
        for (i, ch) in self.line_offsets.iter().enumerate() {
            if ch <= &char {
                continue;
            }
            return i - 1;
        }
        return self.line_offsets.len() - 1;
    }

    fn set_match(&mut self, start: usize, end: usize) {
        self.current_match = Some((start, end));
        let line = self.find_line(start);
        let text = self.text_wrapped.clone();
        let total_chars = text.len();
        self.get_reader()
            .get_inner_mut()
            .set_content(SpannedString::with_spans(
                text,
                vec![
                    IndexedSpan {
                        content: IndexedCow::Borrowed {
                            start: 0,
                            end: start,
                        },
                        attr: Style {
                            effects: enumset::enum_set!(Effect::Simple),
                            color: ColorStyle::inherit_parent(),
                        },
                        width: start,
                    },
                    IndexedSpan {
                        content: IndexedCow::Borrowed { start, end },
                        attr: Style {
                            effects: enumset::enum_set!(Effect::Reverse),
                            color: ColorStyle::inherit_parent(),
                        },
                        width: end - start,
                    },
                    IndexedSpan {
                        content: IndexedCow::Borrowed {
                            start: end,
                            end: total_chars,
                        },
                        attr: Style {
                            effects: enumset::enum_set!(Effect::Simple),
                            color: ColorStyle::inherit_parent(),
                        },
                        width: total_chars - end,
                    },
                ],
            ));

        let reader = self.get_reader();
        if line < reader.content_viewport().top() || line > reader.content_viewport().bottom() {
            let target_line = i64::max(
                0,
                (line as i64) - (reader.content_viewport().height() as i64 / 2),
            ) as usize;
            reader
                .get_scroller_mut()
                .set_offset(Vec2::new(0, target_line));
        }
    }

    fn update_search(&mut self, search_term: &str, next: bool) {
        if let Ok(regex) = Regex::new(&format!("(?i:{})", regex::escape(search_term))) {
            let mut first: Option<(usize, usize)> = None;
            for m in regex.find_iter(&self.text_wrapped) {
                if first.is_none() {
                    first = Some((m.start(), m.end()));
                }
                let current_match_start = self.current_match.map(|m| m.0).unwrap_or(0);
                if m.start() > current_match_start || (!next && m.start() == current_match_start) {
                    self.set_match(m.start(), m.end());
                    // We don't want to trigger wraparound logic and we can reuse `first` instead of having a flag.
                    first = None;
                    break;
                }
            }
            if let Some((start, end)) = first {
                self.set_match(start, end);
            }
        }
    }
}

impl View for ReaderView {
    fn layout(&mut self, size: Vec2) {
        if self.size.x != size.x {
            self.text_wrapped = html2text::from_read(self.html.as_bytes(), size.x - 3);
            let mut line_offsets = vec![0];
            for line in self.text_wrapped.split('\n') {
                line_offsets.push(line_offsets.last().unwrap() + line.len() + 1)
            }
            self.line_offsets = line_offsets;
            let text = self.text_wrapped.clone();
            self.get_reader()
                .get_inner_mut()
                .set_content(SpannedString::<Style>::plain(text));
        }
        self.size = size;
        self.inner.layout(size)
    }

    fn needs_relayout(&self) -> bool {
        self.inner.needs_relayout()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.inner.required_size(constraint)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        let page_scroll_height = i64::max(0, (self.size.y as i64) - 5) as usize;
        if self.inner.get_inner_mut().get_focus_index() == 0 {
            // Reader is focused.
            match event {
                Event::Char(' ') | Event::Key(Key::PageDown) => {
                    self.get_reader()
                        .get_scroller_mut()
                        .scroll_down(page_scroll_height);
                    EventResult::Consumed(None)
                }
                Event::Key(Key::PageUp) => {
                    self.get_reader()
                        .get_scroller_mut()
                        .scroll_up(page_scroll_height);
                    EventResult::Consumed(None)
                }
                Event::Char('/') => {
                    self.get_search().enable();
                    self.inner.get_inner_mut().set_focus_index(1).unwrap();
                    EventResult::Consumed(None)
                }
                Event::Char('q') => EventResult::Consumed(Some(Callback::from_fn(|siv| {
                    let mut stack = get_stack(siv);
                    stack.pop(siv).unwrap();
                }))),
                _ => self.inner.on_event(event),
            }
        } else {
            // Search bar is focused.
            match event {
                Event::Key(Key::Esc) => {
                    self.inner.get_inner_mut().set_focus_index(0).unwrap();
                    let search = self.get_search();
                    search.disable();
                    search.set_content("");
                    let text = self.text_wrapped.clone();
                    self.get_reader().get_inner_mut().set_content(text);
                    EventResult::Consumed(None)
                }
                Event::Key(Key::Enter) => {
                    let search_term = self.get_search().get_content();
                    self.update_search(&search_term.to_string(), true);
                    EventResult::Consumed(None)
                }
                _ => {
                    let old_contents = self.get_search().get_content();
                    let result = self.inner.on_event(event);
                    let new_contents = self.get_search().get_content();
                    if old_contents != new_contents {
                        self.update_search(&new_contents.to_string(), false);
                    }
                    result
                }
            }
        }
    }

    fn call_on_any(&mut self, selector: &Selector, cb: AnyCb) {
        self.inner.call_on_any(selector, cb)
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.take_focus(Direction::none()).unwrap();
        self.inner.focus_view(selector)
    }
    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        self.inner.take_focus(source)
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        self.inner.important_area(view_size)
    }

    fn type_name(&self) -> &'static str {
        "Library Reader View"
    }

    fn draw(&self, printer: &Printer) {
        self.inner.draw(printer);
    }
}
