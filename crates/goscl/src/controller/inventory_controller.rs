use crate::{
    controller::{Controller, ControllerChangeRequest, PlayerInput},
    CursorDirection, GosClientModel, GosClientView, ViewKind,
};
use gems::CycleDirection;

pub struct InventoryController {}

impl Controller for InventoryController {
    fn on_enter(&mut self, _model: &mut GosClientModel, view: &mut GosClientView) {
        view.base_view_kind = ViewKind::Inventory;
        view.overlay_view = None;
    }

    fn on_event(
        &mut self,
        _model: &mut GosClientModel,
        view: &mut GosClientView,
        event: PlayerInput,
    ) -> Option<ControllerChangeRequest> {
        match event {
            PlayerInput::Cursor(direction) => match direction {
                CursorDirection::Up => {
                    view.inventory.select_previous();
                    None
                }
                CursorDirection::Down => {
                    view.inventory.select_next();
                    None
                }
                CursorDirection::Left => {
                    view.inventory
                        .cycle_sorting_criteria(CycleDirection::Backward);
                    None
                }
                CursorDirection::Right => {
                    view.inventory
                        .cycle_sorting_criteria(CycleDirection::Forward);
                    None
                }
            },
            PlayerInput::Enter => Some(ControllerChangeRequest::InventoryItemCommand),
            PlayerInput::Escape => Some(ControllerChangeRequest::AppTerminate),
            _ => None,
        }
    }
}
