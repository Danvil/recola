use crate::mechanics::{colliders::*, switch::*};
use atom::prelude::*;
use candy::{audio::*, can::*, scene_tree::*};
use magi::gems::IntervalF32;

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
        deps.depends_on::<CandyAudioMocca>();
        deps.depends_on::<CandyCanMocca>();
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

const BARRIER_SOUND_RANGE: [f32; 2] = [0.5, 5.000];

fn spawn_barrier(
    mut cmd: Commands,
    asset_resolver: Singleton<SharedAssetResolver>,
    query_tasks: Query<(Entity, &SpawnBarrierTask)>,
) {
    for (door_entity, task) in query_tasks.iter() {
        cmd.entity(door_entity).remove::<SpawnBarrierTask>();

        let audio_path = asset_resolver
            .resolve("audio/effects/sfx-barrier.wav")
            .unwrap();

        cmd.entity(door_entity)
            .and_set(Barrier {
                force_field_entity: task.force_field_entity,
                is_on: true,
            })
            .and_set(AudioSource {
                path: audio_path,
                volume: 1.00,
                state: AudioPlaybackState::Play,
                repeat: AudioRepeatKind::Loop,
                volume_auto_play: false,
            })
            .and_set(SpatialAudioSource {
                range: IntervalF32::from_array(BARRIER_SOUND_RANGE),
                ..Default::default()
            });
    }
}

fn activate_barrier(
    mut cmd: Commands,
    mut query: Query<(Entity, &SwitchObserverState, &mut Barrier, &mut AudioSource)>,
) {
    for (entity, observer, barrier, audio) in query.iter_mut() {
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

            // Change collision behavior of barrier
            cmd.entity(entity).set(ChangeCollidersLayerMaskTask {
                mask: if barrier.is_on {
                    CollisionLayerMask::only_nav()
                } else {
                    CollisionLayerMask::none()
                },
            });

            // toggle audio playback
            audio.volume = if new_on { 1. } else { 0. };
        }
    }
}
