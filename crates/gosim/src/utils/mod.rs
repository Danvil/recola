// mod flecs_query_relation_helpers;
mod newtype_decimal_component;
mod stats;

// pub use flecs_query_relation_helpers::*;

use simplecs::prelude::*;
use std::borrow::Cow;

pub trait EntityBuilder {
    fn build_unamed<'a>(&self, world: &'a mut World) -> EntityWorldMut<'a> {
        let entity = world.spawn_empty();
        let entity = world.entity(entity).unwrap();
        self.build(entity)
    }

    fn new_named<'a>(
        &self,
        world: &'a mut World,
        name: impl Into<Cow<'static, str>>,
    ) -> EntityWorldMut<'a> {
        self.build(world.spawn_empty_named(name))
    }

    fn build<'a>(&self, entity: EntityWorldMut<'a>) -> EntityWorldMut<'a>;

    // fn build_inplace<'a>(&self, entity: &mut EntityWorldMut<'a>) {
    //     *entity = self.build(*entity);
    // }
}
