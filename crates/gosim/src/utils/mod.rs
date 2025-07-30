mod flecs_query_relation_helpers;
mod newtype_decimal_component;
mod stats;
mod test_runner;

pub use flecs_query_relation_helpers::*;
pub use newtype_decimal_component::*;
pub use stats::*;
pub use test_runner::*;

use flecs_ecs::prelude::{EntityView, World};

pub trait EntityBuilder {
    fn new_named<'a, S>(&self, world: &'a World, name: S) -> EntityView<'a>
    where
        S: AsRef<str>,
    {
        self.build(world, world.entity_named(name.as_ref()))
    }

    fn build<'a>(&self, world: &'a World, entity: EntityView<'a>) -> EntityView<'a>;
}
