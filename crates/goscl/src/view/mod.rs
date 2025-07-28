mod body_view;
mod dev_view;
mod entity_list_view;
mod inventory_item_command_view;
mod inventory_view;
mod launcher;

pub use dev_view::*;
pub use entity_list_view::*;
pub use inventory_item_command_view::*;
pub use inventory_view::*;
pub use launcher::*;

use crate::{view::body_view::BodyView, GosClientModel};
use ratatui as rat;

pub trait View {
    fn view(&mut self, model: &GosClientModel, frame: &mut rat::Frame);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    Dev,
    Inventory,
    InventoryItemCommand,
    Body,
}

pub struct GosClientView {
    pub dev: DevView,

    pub inventory: InventoryView,
    pub inventory_item_command: InventoryItemCommandView,
    pub body: BodyView,

    pub base_view_kind: ViewKind,
    pub overlay_view: Option<ViewKind>,
}

impl GosClientView {
    pub fn new() -> Self {
        Self {
            dev: DevView::new(),
            inventory: InventoryView::new(),
            inventory_item_command: InventoryItemCommandView::new(),
            body: BodyView::new(),
            base_view_kind: ViewKind::Dev,
            overlay_view: None,
        }
    }
}

impl View for GosClientView {
    fn view(&mut self, model: &GosClientModel, frame: &mut rat::Frame) {
        match self.base_view_kind {
            ViewKind::Dev => self.dev.view(model, frame),
            ViewKind::Inventory => self.inventory.view(model, frame),
            ViewKind::InventoryItemCommand => panic!(),
            ViewKind::Body => self.body.view(model, frame),
        }

        if let Some(overlay_view) = self.overlay_view {
            match overlay_view {
                ViewKind::Dev => panic!(),
                ViewKind::Inventory => panic!(),
                ViewKind::InventoryItemCommand => self.inventory_item_command.view(model, frame),
                ViewKind::Body => panic!(),
            }
        }
    }
}
