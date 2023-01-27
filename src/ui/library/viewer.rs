use ssh_ui::cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::TextView,
    Cursive, View,
};

pub(crate) fn new_viewer(_siv: &mut Cursive, html: String) -> Box<dyn View> {
    Box::new(
        TextView::new(html2text::from_read(html.as_bytes(), usize::MAX))
            .full_screen()
            .scrollable()
            .with_name("library_viewer"),
    )
}
