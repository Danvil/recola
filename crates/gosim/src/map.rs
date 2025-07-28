use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct MapModule;

#[derive(Component)]
pub struct Tile;

#[derive(Component)]
pub struct LocatedIn;

/// Properties of the air
#[derive(Component, Debug, Clone)]
pub struct Air {
    /// Oxygen content [percent]
    pub oxygen_percent: f64,

    /// Polluting particulate matter [microgram per cubic meter]
    pub pollution: f64,
}

impl Module for MapModule {
    fn module(world: &World) {
        world.module::<MapModule>("map");

        world.component::<Tile>();
        world
            .component::<LocatedIn>()
            .add_trait::<flecs::Transitive>();
    }
}
