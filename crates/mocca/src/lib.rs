use flecs_ecs::prelude::*;

mod deps;
mod runner;

pub use deps::*;
pub use runner::*;

pub trait Mocca: 'static {
    fn load(_dep: MoccaDeps) {}

    fn register_components(_world: &World) {}

    fn start(_world: &World) -> Self;

    fn step(&mut self, _world: &World) {}

    fn fini(&mut self, _world: &World) {}

    fn fini_test(&mut self, _world: &World) {}
}
