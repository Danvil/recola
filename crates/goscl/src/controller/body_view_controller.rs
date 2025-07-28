use crate::{
    controller::{Controller, ControllerChangeRequest, PlayerInput},
    CursorDirection, GosClientModel, GosClientView, ViewKind,
};

pub struct BodyViewController {}

impl BodyViewController {
    pub fn new() -> Self {
        Self {}
    }
}

impl Controller for BodyViewController {
    fn on_enter(&mut self, _model: &mut GosClientModel, view: &mut GosClientView) {
        view.base_view_kind = ViewKind::Body;
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
                    view.body.select_previous();
                    None
                }
                CursorDirection::Down => {
                    view.body.select_next();
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
