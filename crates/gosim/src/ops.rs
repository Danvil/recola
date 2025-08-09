use flecs_ecs::prelude::*;
use mocca::Mocca;

#[derive(Component)]
pub struct OpsMocca;

/// Marker for operations which are not yet completed.
#[derive(Component)]
pub struct OpStatePending;

/// Marker for operations which are completed and ready to be deleted.
#[derive(Component)]
pub struct OpStateCompleted;

impl Mocca for OpsMocca {
    fn register_components(world: &World) {
        world.component::<OpStatePending>();
        world.component::<OpStateCompleted>();
    }

    fn start(_world: &World) -> Self {
        Self
    }

    fn step(&mut self, world: &World) {
        // Delete completed operations
        world
            .query::<()>()
            .with(OpStateCompleted)
            .build()
            .each_entity(|e, _| e.destruct());
    }
}

pub fn op_mark_completed(e: EntityView<'_>) {
    e.remove(OpStatePending);
    e.add(OpStateCompleted);
}

pub fn op_new<Op: ComponentId>(world: &World, op: Op) -> EntityView<'_> {
    world.entity().set(op).add(OpStatePending)
}
