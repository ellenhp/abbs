use regex::Regex;
use ssh_ui::cursive::{
    direction::{Direction, Orientation},
    event::{AnyCb, Callback, Event, EventResult, Key},
    reexports::enumset,
    theme::{ColorStyle, Effect, Style},
    utils::span::{IndexedCow, IndexedSpan, SpannedString},
    view::{CannotFocus, Resizable, Selector, ViewNotFound},
    views::{EditView, LinearLayout, ResizedView, TextView},
    Printer, Rect, Vec2, View,
};

use crate::ui::stack::get_stack;

pub struct ReaderView {
    inner: ResizedView<LinearLayout>,
    size: Vec2,
    html: String,
    text_wrapped: String,
    line_offsets: Vec<usize>,
    char_offset: usize,
    current_match: Option<(usize, usize)>,
}

impl ReaderView {
    pub fn new(html: &str) -> ReaderView {
        let reader = TextView::new("Loading...").full_screen();
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
            char_offset: 0,
            current_match: None,
        }
    }

    fn get_reader<'a>(&'a mut self) -> &'a mut TextView {
        self.inner
            .get_inner_mut()
            .get_child_mut(0)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<ResizedView<TextView>>()
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

    fn scroll_by_lines(&mut self, lines: i32) {
        let line = self.find_line(self.char_offset);
        let target_line = i32::clamp(line as i32 + lines, 0, self.line_offsets.len() as i32).clamp(
            0,
            i32::max(0, self.line_offsets.len() as i32 - self.size.y as i32),
        );
        self.scroll_to_line(target_line as usize);
    }

    fn scroll_to_line(&mut self, line: usize) {
        self.char_offset = self.line_offsets[line.clamp(0, self.line_offsets.len() - 1)];
    }

    fn set_match(&mut self, start: usize, end: usize) {
        self.current_match = Some((start, end));
        let text = self.text_wrapped.clone();
        let total_chars = text.len();
        self.get_reader().set_content(SpannedString::with_spans(
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
        }
        let current_line = self.find_line(self.char_offset);

        let end_char = if current_line + size.y >= self.line_offsets.len() {
            self.text_wrapped.len()
        } else {
            self.line_offsets[current_line + size.y]
        };
        let end_char = end_char.clamp(0, self.text_wrapped.len());
        let char_offset = self.char_offset.clamp(0, self.text_wrapped.len());
        let wrapped_string = self.text_wrapped[char_offset..end_char].to_string();
        self.get_reader().set_content(wrapped_string);
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
                    self.scroll_by_lines(page_scroll_height as i32);
                    EventResult::Consumed(None)
                }
                Event::Key(Key::PageUp) => {
                    self.scroll_by_lines(-(page_scroll_height as i32));
                    EventResult::Consumed(None)
                }
                Event::Key(Key::Down) => {
                    self.scroll_by_lines(1);
                    EventResult::Consumed(None)
                }
                Event::Key(Key::Up) => {
                    self.scroll_by_lines(-1);
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
                    self.get_reader().set_content(text);
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
