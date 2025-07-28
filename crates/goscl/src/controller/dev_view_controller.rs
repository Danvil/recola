use crate::{
    controller::{Controller, ControllerChangeRequest, PlayerInput},
    CursorDirection, GosClientModel, GosClientView, ViewKind,
};

pub struct DevViewController {}

impl DevViewController {
    pub fn new() -> Self {
        Self {}
    }
}

impl Controller for DevViewController {
    fn on_enter(&mut self, _model: &mut GosClientModel, view: &mut GosClientView) {
        view.base_view_kind = ViewKind::Dev;
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
                    view.dev.select_previous();
                    None
                }
                CursorDirection::Down => {
                    view.dev.select_next();
                    None
                }
                _ => None, // no left/right support
            },
            PlayerInput::Enter => {
                // Placeholder: adjust if needed to trigger organ actions
                None
            }
            PlayerInput::Escape => Some(ControllerChangeRequest::AppTerminate),
            _ => None,
        }
    }
}
