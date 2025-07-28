use crate::{
    controller::{
        body_view_controller::BodyViewController, dev_view_controller::DevViewController,
        inventory_controller::InventoryController,
        inventory_item_command_controller::InventoryItemCommandController,
    },
    Controller, ControllerChangeRequest, GosClientModel, GosClientView, PlayerInput,
};

pub struct ControllerDispatch {
    wants_to_quit: bool,
    active: ControllerKind,
    dev_view_ctrl: DevViewController,
    inventory_ctrl: InventoryController,
    inventory_item_command_ctrl: InventoryItemCommandController,
    organs_ctrl: BodyViewController,
}

#[derive(Clone, PartialEq)]
enum ControllerKind {
    DevView,
    Inventory,
    InventoryItemCommand,
    BodyView,
}

impl ControllerDispatch {
    pub fn new() -> Self {
        Self {
            wants_to_quit: false,
            active: ControllerKind::DevView,
            dev_view_ctrl: DevViewController {},
            inventory_ctrl: InventoryController {},
            inventory_item_command_ctrl: InventoryItemCommandController {},
            organs_ctrl: BodyViewController::new(),
        }
    }

    pub fn transition(
        &mut self,
        change_request: ControllerChangeRequest,
        model: &mut GosClientModel,
        view: &mut GosClientView,
    ) {
        self.on_exit(model, view);
        self.active = match change_request {
            ControllerChangeRequest::DevView => ControllerKind::DevView,
            ControllerChangeRequest::Inventory => ControllerKind::Inventory,
            ControllerChangeRequest::InventoryItemCommand => ControllerKind::InventoryItemCommand,
            ControllerChangeRequest::BodyView => ControllerKind::BodyView,
            ControllerChangeRequest::AppTerminate => {
                self.wants_to_quit = true;
                return;
            }
        };
        self.on_enter(model, view);
    }

    pub fn wants_to_quit(&self) -> bool {
        self.wants_to_quit
    }
}

impl Controller for ControllerDispatch {
    fn on_enter(&mut self, model: &mut GosClientModel, view: &mut GosClientView) {
        match &mut self.active {
            ControllerKind::DevView => self.dev_view_ctrl.on_enter(model, view),
            ControllerKind::Inventory => self.inventory_ctrl.on_enter(model, view),
            ControllerKind::InventoryItemCommand => {
                self.inventory_item_command_ctrl.on_enter(model, view)
            }
            ControllerKind::BodyView => self.organs_ctrl.on_enter(model, view),
        }
    }

    fn on_exit(&mut self, model: &mut GosClientModel, view: &mut GosClientView) {
        match &mut self.active {
            ControllerKind::DevView => self.dev_view_ctrl.on_exit(model, view),
            ControllerKind::Inventory => self.inventory_ctrl.on_exit(model, view),
            ControllerKind::InventoryItemCommand => {
                self.inventory_item_command_ctrl.on_exit(model, view)
            }
            ControllerKind::BodyView => self.organs_ctrl.on_exit(model, view),
        }
    }

    fn on_event(
        &mut self,
        model: &mut GosClientModel,
        view: &mut GosClientView,
        event: PlayerInput,
    ) -> Option<ControllerChangeRequest> {
        match &mut self.active {
            ControllerKind::DevView => self.dev_view_ctrl.on_event(model, view, event),
            ControllerKind::Inventory => self.inventory_ctrl.on_event(model, view, event),
            ControllerKind::InventoryItemCommand => self
                .inventory_item_command_ctrl
                .on_event(model, view, event),
            ControllerKind::BodyView => self.organs_ctrl.on_event(model, view, event),
        }
    }
}
