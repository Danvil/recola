use crate::{
    controller::{Controller, ControllerChangeRequest, PlayerInput},
    CursorDirection, GosClientModel, GosClientView, ViewKind,
};

pub struct InventoryItemCommandController {}

impl Controller for InventoryItemCommandController {
    fn on_enter(&mut self, _model: &mut GosClientModel, view: &mut GosClientView) {
        view.overlay_view = Some(ViewKind::InventoryItemCommand);
    }

    fn on_event(
        &mut self,
        _model: &mut GosClientModel,
        _view: &mut GosClientView,
        event: PlayerInput,
    ) -> Option<ControllerChangeRequest> {
        match event {
            PlayerInput::Cursor(direction) => match direction {
                CursorDirection::Up => {
                    // view.player_inventory.select_previous();
                    None
                }
                CursorDirection::Down => {
                    // view.player_inventory.select_next();
                    None
                }
                CursorDirection::Left => None,
                CursorDirection::Right => None,
            },
            PlayerInput::Enter => {
                // TODO
                None
            }
            PlayerInput::Escape => Some(ControllerChangeRequest::Inventory),
            _ => None,
        }
    }
}
