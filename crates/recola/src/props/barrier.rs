use crate::mechanics::{colliders::*, switch::*};
use atom::prelude::*;
use candy::scene_tree::*;

#[derive(Component)]
pub struct SpawnBarrierTask {
    /// This entity is made invisible if the barrier deactivates
    pub force_field_entity: Entity,

    /// The collider entity blocking passage
    pub collider_entity: Entity,
}

#[derive(Component, Debug, Clone)]
pub struct Barrier {
    force_field_entity: Entity,
    collider_entity: Entity,
    is_on: bool,
}

/// Laser pointers with a beam which collides with objects
pub struct BarrierMocca;

impl Mocca for BarrierMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<SwitchMocca>();
        deps.depends_on::<CollidersMocca>();
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
            collider_entity: task.collider_entity,
            is_on: true,
        });
    }
}

fn activate_barrier(
    mut cmd: Commands,
    mut query: Query<(Entity, &SwitchObserverState, &mut Barrier)>,
    mut query_collider: Query<&mut CollisionLayerMask>,
) {
    for (entity, observer, barrier) in query.iter_mut() {
        let new_on = !observer.as_bool();

        if new_on != barrier.is_on {
            barrier.is_on = new_on;

            if barrier.is_on {
                log::debug!("barrier {entity} is ON");

                cmd.entity(barrier.force_field_entity)
                    .and_set(Visibility::Visible);
            } else {
                log::debug!("barrier {entity} is OFF");

                cmd.entity(barrier.force_field_entity)
                    .and_set(Visibility::Hidden);
            }

            query_collider.get_mut(barrier.collider_entity).unwrap().nav = barrier.is_on;

            cmd.entity(barrier.collider_entity)
                .set(DirtyCollider::default());
        }
    }
}
