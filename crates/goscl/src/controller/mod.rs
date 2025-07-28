use crate::{controller::dispatch::ControllerDispatch, GosClientModel, GosClientView};
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};

mod body_view_controller;
mod dev_view_controller;
mod dispatch;
mod inventory_controller;
mod inventory_item_command_controller;

pub trait Controller {
    fn on_enter(&mut self, _model: &mut GosClientModel, _view: &mut GosClientView) {}

    fn on_exit(&mut self, _model: &mut GosClientModel, _view: &mut GosClientView) {}

    fn on_event(
        &mut self,
        model: &mut GosClientModel,
        view: &mut GosClientView,
        event: PlayerInput,
    ) -> Option<ControllerChangeRequest>;
}

pub enum ControllerChangeRequest {
    DevView,
    AppTerminate,
    Inventory,
    InventoryItemCommand,
    BodyView,
}

#[derive(Debug, PartialEq)]
pub enum PlayerInput {
    Tick,
    Cursor(CursorDirection),
    Enter,
    Escape,
    Backspace,
    Tab,
    Inspect,
    AlphaNum(char),
}

#[derive(Debug, PartialEq)]
pub enum CursorDirection {
    Down,
    Left,
    Right,
    Up,
}

pub struct GosClientController {
    dispatch: ControllerDispatch,
}

impl GosClientController {
    pub fn new() -> Self {
        Self {
            dispatch: ControllerDispatch::new(),
        }
    }

    pub fn on_win_event(
        &mut self,
        model: &mut GosClientModel,
        view: &mut GosClientView,
        event: Event,
    ) {
        let mut maybe_event = None;
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => maybe_event = Some(PlayerInput::Escape),
                    KeyCode::Enter => maybe_event = Some(PlayerInput::Enter),
                    KeyCode::Backspace => maybe_event = Some(PlayerInput::Backspace),
                    KeyCode::Tab => maybe_event = Some(PlayerInput::Tab),
                    KeyCode::F(1) => {
                        self.dispatch
                            .transition(ControllerChangeRequest::Inventory, model, view);
                    }
                    KeyCode::Char(ch) => {
                        if ch.is_alphanumeric() {
                            maybe_event = Some(PlayerInput::AlphaNum(ch))
                        } else if ch == '?' {
                            maybe_event = Some(PlayerInput::Inspect);
                        }
                    }
                    KeyCode::Left => maybe_event = Some(PlayerInput::Cursor(CursorDirection::Left)),
                    KeyCode::Right => {
                        maybe_event = Some(PlayerInput::Cursor(CursorDirection::Right))
                    }
                    KeyCode::Up => maybe_event = Some(PlayerInput::Cursor(CursorDirection::Up)),
                    KeyCode::Down => maybe_event = Some(PlayerInput::Cursor(CursorDirection::Down)),
                    _ => {}
                }
            }
        }

        if let Some(event) = maybe_event {
            let maybe_ctrl_change_request = self.dispatch.on_event(model, view, event);

            if let Some(ctrl_change_request) = maybe_ctrl_change_request {
                self.dispatch.transition(ctrl_change_request, model, view);
            }
        }
    }

    pub fn on_tick(&mut self, model: &mut GosClientModel, view: &mut GosClientView) {
        self.dispatch.on_event(model, view, PlayerInput::Tick);
    }

    pub fn wants_to_quit(&self) -> bool {
        self.dispatch.wants_to_quit()
    }
}
