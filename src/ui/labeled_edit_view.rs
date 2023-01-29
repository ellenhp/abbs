use ssh_ui::cursive::{
    direction::Direction,
    event::{AnyCb, Event, EventResult},
    view::{CannotFocus, IntoBoxedView, Margins, Nameable, Resizable, Selector, ViewNotFound},
    views::{EditView, LinearLayout, PaddedView, ResizedView, TextView},
    Cursive, Printer, Rect, Vec2, View,
};

pub struct LabeledEditView {
    inner: Box<dyn View>,
}

impl LabeledEditView {
    pub fn new<EditCb, SubmitCb>(
        label: &str,
        min_label_width: Option<usize>,
        initial_value: &str,
        edit_cb: EditCb,
        submit_cb: SubmitCb,
        edit_view_name: &str,
    ) -> LabeledEditView
    where
        EditCb: Fn(&mut Cursive, &str, usize) + 'static,
        SubmitCb: Fn(&mut Cursive, &str) + 'static,
    {
        let label = TextView::new(label).min_width(min_label_width.unwrap_or(0));
        let edit = EditView::new()
            .content(initial_value)
            .on_edit_mut(edit_cb)
            .on_submit_mut(submit_cb)
            .with_name(edit_view_name)
            .full_width();
        LabeledEditView {
            inner: LinearLayout::horizontal()
                .child(PaddedView::new(Margins::lrtb(0, 1, 0, 0), label))
                .child(edit)
                .full_width()
                .into_boxed_view(),
        }
    }
}

impl View for LabeledEditView {
    fn layout(&mut self, size: Vec2) {
        self.inner.layout(size)
    }

    fn needs_relayout(&self) -> bool {
        self.inner.needs_relayout()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.inner.required_size(constraint)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        self.inner.on_event(event)
    }

    fn call_on_any(&mut self, selector: &Selector, cb: AnyCb) {
        self.inner.call_on_any(selector, cb)
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<EventResult, ViewNotFound> {
        self.take_focus(Direction::none()).unwrap();
        self.inner.focus_view(selector)
    }
    fn take_focus(&mut self, source: Direction) -> Result<EventResult, CannotFocus> {
        self.inner
            .as_any_mut()
            .downcast_mut::<ResizedView<LinearLayout>>()
            .unwrap()
            .get_inner_mut()
            .get_child_mut(1)
            .unwrap()
            .take_focus(Direction::none())
            .unwrap();
        self.inner.take_focus(source)
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        self.inner.important_area(view_size)
    }

    fn type_name(&self) -> &'static str {
        "Labeled EditView"
    }

    fn draw(&self, printer: &Printer) {
        self.inner.draw(printer)
    }
}
