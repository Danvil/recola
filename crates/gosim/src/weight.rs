use crate::decimal_component;
use flecs_ecs::prelude::{Component, Module, World};

/// The weight of a physical object
decimal_component!(Weight);

#[derive(Component)]
pub struct WeightModule;

impl Module for WeightModule {
    fn module(world: &World) {
        world.module::<WeightModule>("WeightModule");

        world.component::<Weight>();
    }
}
