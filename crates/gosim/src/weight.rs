use crate::{decimal_component, ecs::prelude::*};

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
