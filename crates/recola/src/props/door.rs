use crate::{
    collision::*,
    custom_properties::*,
    mechanics::{material_swap::*, switch::*},
    player::*,
    recola_mocca::CRIMSON,
};
use atom::prelude::*;
use candy::{audio::*, can::*, scene_tree::*, time::*};
use eyre::{Result, eyre};
use magi::{
    bsdf::PbrMaterial,
    gems::{IntervalF32, SmoothInputControl, SmoothInputF32, SmoothInputF32Settings},
};

/// Creates a new gate which can be lowered by the player if they have the right key
#[derive(Component)]
pub struct SpawnLevelGateTask {
    pub relief_entity: Entity,
}

/// Crates a new double sliding door which is opened when powered
#[derive(Component)]
pub struct SpawnDoubleDoorTask {
    pub leafes: [(Entity, f32); 2],
    pub colliders: [(Entity, f32); 2],
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyId(pub i64);

/// Laser pointers with a beam which collides with objects
pub struct DoorMocca;

impl Mocca for DoorMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyAudioMocca>();
        deps.depends_on::<CandyCanMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<CustomPropertiesMocca>();
        deps.depends_on::<MaterialSwapMocca>();
        deps.depends_on::<PlayerMocca>();
        deps.depends_on::<SwitchMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<DoubleDoor>();
        world.register_component::<GlowOnKey>();
        world.register_component::<KeyId>();
        world.register_component::<LevelGate>();
        world.register_component::<SpawnDoubleDoorTask>();
        world.register_component::<SpawnLevelGateTask>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(spawn_level_gate);
        world.run(level_gate_glow_on_key);
        world.run(leve_gate_interaction);
        world.run(lower_level_gate);

        world.run(spawn_double_door);
        world.run(open_double_door);
    }
}

#[derive(Component, Debug, Clone)]
struct LevelGate {
    lower_progress: f32,
    progress_changed: bool,
    is_lowered: bool,
}

const LEVEL_GATE_INTERACTION_DISTANCE: f32 = 5.;
const LEVEL_GATE_LOWER_MAX: f32 = 3.933;
const LEVEL_GATE_LOWER_DURATION: f32 = 5.5; // TODO should match audio clip length!
const LEVEL_GATE_LOWER_SPEED: f32 = LEVEL_GATE_LOWER_MAX / LEVEL_GATE_LOWER_DURATION;
const LEVEL_GATE_LOWER_SPEED_SOUND_RANGE: [f32; 2] = [1.000, 20.000];

fn spawn_level_gate(
    mut cmd: Commands,
    asset_resolver: Singleton<SharedAssetResolver>,
    query_open_door_task: Query<(Entity, &SpawnLevelGateTask)>,
    query_props: Query<&CustomProperties>,
) {
    let door_open_clip = asset_resolver
        .resolve("audio/effects/sfx-level_gate.wav")
        .unwrap();

    for (door_entity, task) in query_open_door_task.iter() {
        cmd.entity(door_entity).remove::<SpawnLevelGateTask>();

        let key_id = match get_key_id(&query_props, door_entity) {
            Ok(key_id) => key_id,
            Err(err) => {
                log::error!("door without key_id: {err:?}");
                continue;
            }
        };

        cmd.entity(door_entity)
            .and_set(DynamicTransform)
            .and_set(LevelGate {
                lower_progress: 0.,
                progress_changed: false,
                is_lowered: false,
            })
            .and_set(key_id)
            .and_set(GlowOnKey {
                relief_entity: task.relief_entity,
            })
            .and_set(AudioSource {
                path: door_open_clip.clone(),
                volume: 0.85,
                state: AudioPlaybackState::Stop,
                repeat: AudioRepeatKind::Stop,
                volume_auto_play: false,
            })
            .and_set(SpatialAudioSource {
                range: IntervalF32::from_array(LEVEL_GATE_LOWER_SPEED_SOUND_RANGE),
                ..Default::default()
            });

        cmd.entity(task.relief_entity)
            .and_set(MaterialSwap::from_iter([
                PbrMaterial::diffuse(CRIMSON),
                PbrMaterial::diffuse(CRIMSON).with_emission(CRIMSON.to_linear() * 3.33),
            ]))
            .and_set(MaterialSwapTransition::ZERO);
    }
}

fn get_key_id(query_props: &Query<&CustomProperties>, door_entity: Entity) -> Result<KeyId> {
    let props = query_props
        .get(door_entity)
        .ok_or_else(|| eyre!("door does not have CustomProperties"))?;

    let id = props
        .get_integer("key_id")
        .ok_or_else(|| eyre!("'key_id' entry missing"))?;

    Ok(KeyId(id))
}

fn leve_gate_interaction(
    time: Singleton<SimClock>,
    player: Singleton<Player>,
    query_input_raycast: Query<&InputRaycastController>,
    mut query_door: Query<(&mut LevelGate, &KeyId)>,
) {
    let dt = time.sim_dt_f32();
    let input_raycast = &query_input_raycast.single().unwrap();

    // Charge when mouse is pressed
    if !input_raycast.state().is_left_mouse_pressed {
        return;
    }

    // Get hit entity
    let Some((hit_entity, distance)) = input_raycast.raycast_entity_and_distance() else {
        return;
    };

    // Check we are close enough
    if distance > LEVEL_GATE_INTERACTION_DISTANCE {
        return;
    }

    // Get door
    let Some((door, key)) = query_door.get_mut(hit_entity) else {
        return;
    };

    // Do not operate open door
    if door.is_lowered {
        return;
    }

    // Operate door
    if player.keys.contains(key) {
        door.lower_progress += LEVEL_GATE_LOWER_SPEED * dt;
        if door.lower_progress >= LEVEL_GATE_LOWER_MAX {
            door.lower_progress = LEVEL_GATE_LOWER_MAX;
            door.is_lowered = true;
        }
        door.progress_changed = true;
    } else {
        log::debug!("missing key {key:?}");
    }
}

#[derive(Component)]
pub struct GlowOnKey {
    relief_entity: Entity,
}

fn level_gate_glow_on_key(
    mut cmd: Commands,
    player: Singleton<Player>,
    mut query_door: Query<(Entity, &mut GlowOnKey, &KeyId)>,
) {
    for (door_entity, glow, key) in query_door.iter_mut() {
        if player.keys.contains(key) {
            // initiate material transition
            cmd.entity(glow.relief_entity)
                .and_set(MaterialSwapTransition {
                    index: 1,
                    speed: 0.133,
                });

            // and remove component
            cmd.entity(door_entity).remove::<GlowOnKey>();
        }
    }
}

fn lower_level_gate(
    mut cmd: Commands,
    mut query_door: Query<(Entity, &mut Transform3, &mut LevelGate, &mut AudioSource)>,
) {
    for (door_entity, tf, door, audio) in query_door.iter_mut() {
        // move door down
        if door.progress_changed {
            tf.translation.z = -door.lower_progress;

            if door.is_lowered {
                log::debug!("door {door_entity} lowered");
                cmd.entity(door_entity)
                    .and_set(ChangeCollidersLayerMaskTask {
                        mask: CollisionLayerMask::none(),
                    });
            }
        }

        // play audio while operating
        audio.state = if door.progress_changed {
            AudioPlaybackState::Play
        } else {
            AudioPlaybackState::Pause
        };

        door.progress_changed = false;
    }
}

#[derive(Component, Debug, Clone)]
struct DoubleDoor {
    leafes: [(Entity, f32); 2],
    colliders: [(Entity, f32); 2],
    open_progress: SmoothInputF32,
}

const DOUBLE_DOOR_OPEN_DELTA: f32 = 1.677;
const DOUBLE_DOOR_OPEN_DURATION: f32 = 3.000;
const DOUBLE_DOOR_OPEN_SETTINGS: SmoothInputF32Settings = SmoothInputF32Settings {
    value_range: Some((0., 1.)),
    max_speed: DOUBLE_DOOR_OPEN_DELTA / DOUBLE_DOOR_OPEN_DURATION,
    max_accel: 1.,
    max_deaccel: 1.,
};
const DOUBLE_DOOR_SOUND_RANGE: [f32; 2] = [1.000, 20.000];

fn spawn_double_door(
    mut cmd: Commands,
    asset_resolver: Singleton<SharedAssetResolver>,
    query_open_door_task: Query<(Entity, &SpawnDoubleDoorTask)>,
) {
    let door_open_clip = asset_resolver
        .resolve("audio/effects/sfx-double_door.wav")
        .unwrap();

    for (door_entity, task) in query_open_door_task.iter() {
        cmd.entity(door_entity).remove::<SpawnDoubleDoorTask>();

        cmd.entity(door_entity)
            .and_set(DynamicTransform)
            .and_set(DoubleDoor {
                leafes: task.leafes,
                colliders: task.colliders,
                open_progress: SmoothInputF32::default(),
            })
            .and_set(AudioSource {
                path: door_open_clip.clone(),
                volume: 0.95,
                state: AudioPlaybackState::Stop,
                repeat: AudioRepeatKind::Loop,
                volume_auto_play: true,
            })
            .and_set(SpatialAudioSource {
                range: IntervalF32::from_array(DOUBLE_DOOR_SOUND_RANGE),
                ..Default::default()
            });

        log::debug!("spawned double door: {door_entity}");
    }
}

fn open_double_door(
    mut cmd: Commands,
    time: Singleton<SimClock>,
    mut query_door: Query<(
        Entity,
        &SwitchObserverState,
        &mut DoubleDoor,
        &mut AudioSource,
    )>,
    mut query_tf: Query<&mut Transform3>,
) {
    let dt = time.sim_dt_f32();

    for (door_entity, switch_observer, door, audio) in query_door.iter_mut() {
        // open door if powered
        let has_power = switch_observer.as_bool();
        door.open_progress.update(
            dt,
            &DOUBLE_DOOR_OPEN_SETTINGS,
            if has_power {
                SmoothInputControl::Increase
            } else {
                SmoothInputControl::Decrease
            },
            1.,
        );
        log::trace!(
            "double door power: {has_power} {}",
            door.open_progress.value()
        );

        // slide doors open
        let delta = DOUBLE_DOOR_OPEN_DELTA * door.open_progress.value();
        for (&(entity, y0), dir) in door.leafes.iter().zip([1.0, -1.0]) {
            query_tf.get_mut(entity).unwrap().translation.y = y0 + dir * delta;
        }

        // update colliders
        for (&(entity, y0), dir) in door.colliders.iter().zip([1.0, -1.0]) {
            query_tf.get_mut(entity).unwrap().translation.y = y0 + dir * delta;
        }
        cmd.entity(door_entity).and_set(CollidersDirtyTask);

        // update audio
        audio.volume = IntervalF32::from_min_max(0.01, 0.3)
            .rescale_unit_clamped(door.open_progress.velocity.abs());
    }
}
