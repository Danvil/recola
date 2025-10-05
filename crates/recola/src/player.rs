use excess::prelude::*;
use simplecs::prelude::*;

#[derive(Singleton)]
pub struct Player {
    pub rift_charges: usize,
}

/// Laser pointers with a beam which collides with objects
pub struct PlayerMocca;

impl Mocca for PlayerMocca {
    fn load(mut deps: MoccaDeps) {}

    fn start(world: &mut World) -> Self {
        world.set_singleton(Player { rift_charges: 0 });
        Self
    }

    fn register_components(world: &mut World) {}

    fn step(&mut self, world: &mut World) {}
}
