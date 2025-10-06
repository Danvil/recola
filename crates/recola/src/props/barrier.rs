use crate::switch::{SwitchMocca, SwitchObserverState};
use candy_scene_tree::{CandySceneTreeMocca, Visibility};
use excess::prelude::*;
use simplecs::prelude::*;

#[derive(Component)]
pub struct SpawnBarrierTask {
    /// This entity is made invisible if the barrier deactivates
    pub force_field_entity: Entity,
}

#[derive(Component, Debug, Clone)]
pub struct Barrier {
    force_field_entity: Entity,
    is_on: bool,
}

/// Laser pointers with a beam which collides with objects
pub struct BarrierMocca;

impl Mocca for BarrierMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<SwitchMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<SpawnBarrierTask>();
        world.register_component::<Barrier>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(spawn_barrier);
        world.run(activate_barrier);
    }
}

fn spawn_barrier(mut cmd: Commands, query_tasks: Query<(Entity, &SpawnBarrierTask)>) {
    for (door_entity, task) in query_tasks.iter() {
        cmd.entity(door_entity).remove::<SpawnBarrierTask>();
        cmd.entity(door_entity).and_set(Barrier {
            force_field_entity: task.force_field_entity,
            is_on: true,
        });
    }
}

fn activate_barrier(mut cmd: Commands, mut query: Query<(&SwitchObserverState, &mut Barrier)>) {
    for (observer, barrier) in query.iter_mut() {
        barrier.is_on = !observer.as_bool();

        if barrier.is_on {
            cmd.entity(barrier.force_field_entity)
                .and_set(Visibility::Visible);
        } else {
            cmd.entity(barrier.force_field_entity)
                .and_set(Visibility::Hidden);
        }
    }
}
