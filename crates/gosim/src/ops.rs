use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct OpsModule;

/// Marker for operations which are not yet completed.
#[derive(Component)]
pub struct OpStatePending;

/// Marker for operations which are completed and ready to be deleted.
#[derive(Component)]
pub struct OpStateCompleted;

impl Module for OpsModule {
    fn module(world: &World) {
        world.module::<OpsModule>("ops");

        world.component::<OpStatePending>();
        world.component::<OpStateCompleted>();

        // Delete completed operations
        world
            .system::<()>()
            .with(OpStateCompleted)
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
