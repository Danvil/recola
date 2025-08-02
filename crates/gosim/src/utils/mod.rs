mod flecs_query_relation_helpers;
mod newtype_decimal_component;
mod stats;

pub use flecs_query_relation_helpers::*;
pub use newtype_decimal_component::*;
pub use stats::*;

use flecs_ecs::prelude::{EntityView, World};

pub trait EntityBuilder {
    fn build_unamed<'a>(&self, world: &'a World) -> EntityView<'a> {
        self.build(world, world.entity())
    }

    fn new_named<'a, S>(&self, world: &'a World, name: S) -> EntityView<'a>
    where
        S: AsRef<str>,
    {
        self.build(world, world.entity_named(name.as_ref()))
    }

    fn build<'a>(&self, world: &'a World, entity: EntityView<'a>) -> EntityView<'a>;
}
