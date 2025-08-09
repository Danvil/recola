use crate::decimal_component;
use flecs_ecs::prelude::{Component, World};
use mocca::Mocca;

/// The weight of a physical object
decimal_component!(Weight);

#[derive(Component)]
pub struct WeightMocca;

impl Mocca for WeightMocca {
    fn register_components(world: &World) {
        world.component::<Weight>();
    }

    fn start(_world: &World) -> Self {
        Self
    }
}
