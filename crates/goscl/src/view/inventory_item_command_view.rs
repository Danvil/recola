use crate::{GosClientModel, View};
use ratatui::{
    prelude::{Rect, Stylize},
    style::Style,
    widgets::{Block, Clear, List, ListDirection},
    Frame,
};

pub struct InventoryItemCommandView {}

impl InventoryItemCommandView {
    pub fn new() -> Self {
        Self {}
    }
}

impl View for InventoryItemCommandView {
    fn view(&mut self, _model: &GosClientModel, frame: &mut Frame) {
        let width = 10;
        let height = 10;
        let popup_area = Rect {
            x: frame.area().x + frame.area().width / 2 - width / 2,
            y: frame.area().y + frame.area().height / 2 - height / 2,
            width,
            height,
        };

        frame.render_widget(Clear, popup_area);

        let items = ["Item 1", "Item 2", "Item 3"];
        let list = List::new(items)
            .block(Block::bordered().title("List"))
            .style(Style::new().white())
            .highlight_style(Style::new().italic())
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::BottomToTop);

        frame.render_widget(&list, popup_area);
    }
}
