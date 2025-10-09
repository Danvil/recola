use atom::prelude::*;
use std::collections::HashSet;

/// Observes switches and updates accordingly
#[derive(Component)]
pub struct SwitchObserver {
    /// Observer is active if all these switches are on
    pub switches: Vec<String>,

    /// If enabled the observer will stay active once activated even if a switch is turned off
    pub latch: bool,
}

/// Activation state of a switch observer
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum SwitchObserverState {
    Active,
    Inactive,
}

impl SwitchObserverState {
    pub fn as_bool(&self) -> bool {
        *self == SwitchObserverState::Active
    }
}

/// A switch
#[derive(Component)]
pub struct Switch {
    pub name: String,
}

/// The state of a switch
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum SwitchState {
    On,
    Off,
}

impl SwitchState {
    pub fn as_bool(&self) -> bool {
        *self == SwitchState::On
    }

    pub fn set_from_bool(&mut self, value: bool) {
        *self = Self::from_bool(value);
    }

    pub fn from_bool(value: bool) -> Self {
        if value {
            SwitchState::On
        } else {
            SwitchState::Off
        }
    }
}

/// Switches and switch observers
pub struct SwitchMocca;

impl Mocca for SwitchMocca {
    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<SwitchObserver>();
        world.register_component::<SwitchObserverState>();
        world.register_component::<Switch>();
        world.register_component::<SwitchState>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(update_switch_triggers);
    }
}

fn update_switch_triggers(
    query_switches: Query<(&Switch, &SwitchState)>,
    mut query_observers: Query<(Entity, &SwitchObserver, &mut SwitchObserverState)>,
) {
    let mut active_switches = HashSet::new();

    for (switch, state) in query_switches.iter() {
        if state.as_bool() {
            active_switches.insert(switch.name.as_str());
        }
    }
    log::trace!("active switches: {:?}", active_switches);

    for (entity, observer, state) in query_observers.iter_mut() {
        log::trace!("Processing observer {:?}: {:?}", entity, observer.switches);

        let active = observer
            .switches
            .iter()
            .all(|id| active_switches.contains(id.as_str()));

        if active {
            if !state.as_bool() {
                log::debug!("activated switch observer {entity:?}");
            }
            *state = SwitchObserverState::Active;
        } else {
            if !observer.latch {
                if state.as_bool() {
                    log::debug!("de-activated switch observer {entity:?}");
                }
                *state = SwitchObserverState::Inactive;
            }
        }
    }
}
