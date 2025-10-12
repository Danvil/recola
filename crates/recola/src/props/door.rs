use crate::{
    custom_properties::*,
    mechanics::{colliders::*, material_swap::*, switch::*},
    player::*,
    recola_mocca::CRIMSON,
};
use atom::prelude::*;
use candy::{scene_tree::*, time::*};
use eyre::{Result, eyre};
use magi::{
    bsdf::PbrMaterial,
    gems::{SmoothInputF32, SmoothInputF32Settings},
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
        world.register_component::<KeyId>();
        world.register_component::<LevelGate>();
        world.register_component::<SpawnDoubleDoorTask>();
        world.register_component::<SpawnLevelGateTask>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(spawn_level_gate);
        world.run(leve_gate_interaction);
        world.run(lower_level_gate);
        world.run(spawn_double_door);
        world.run(open_double_door);
    }
}

#[derive(Component, Debug, Clone)]
struct LevelGate {
    relief_entity: Entity,
    lower_progress: f32,
    progress_changed: bool,
    is_lowered: bool,
}

const LEVEL_GATE_INTERACTION_DISTANCE: f32 = 5.;
const LEVEL_GATE_LOWER_SPEED: f32 = 1.333;
const LEVEL_GATE_LOWER_MAX: f32 = 3.933;

fn spawn_level_gate(
    mut cmd: Commands,
    query_open_door_task: Query<(Entity, &SpawnLevelGateTask)>,
    query_props: Query<&CustomProperties>,
) {
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
                relief_entity: task.relief_entity,
                lower_progress: 0.,
                progress_changed: false,
                is_lowered: false,
            })
            .and_set(key_id);

        cmd.entity(task.relief_entity)
            .and_set(MaterialSwap::from_iter([
                PbrMaterial::diffuse(CRIMSON),
                PbrMaterial::diffuse(CRIMSON).with_emission(CRIMSON.to_linear() * 3.0),
            ]))
            .and_set(MaterialSwapSelection(0));
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

fn lower_level_gate(
    mut cmd: Commands,
    mut query_door: Query<(Entity, &mut Transform3, &mut LevelGate)>,
) {
    for (door_entity, tf, door) in query_door.iter_mut() {
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

        // change material while operating
        cmd.entity(door.relief_entity)
            .and_set(MaterialSwapSelection::from_bool(door.progress_changed));

        door.progress_changed = false;
    }
}

#[derive(Component, Debug, Clone)]
struct DoubleDoor {
    leafes: [(Entity, f32); 2],
    colliders: [(Entity, f32); 2],
    open_progress: SmoothInputF32,
}

fn spawn_double_door(
    mut cmd: Commands,
    query_open_door_task: Query<(Entity, &SpawnDoubleDoorTask)>,
) {
    for (door_entity, task) in query_open_door_task.iter() {
        cmd.entity(door_entity).remove::<SpawnDoubleDoorTask>();

        cmd.entity(door_entity)
            .and_set(DynamicTransform)
            .and_set(DoubleDoor {
                leafes: task.leafes,
                colliders: task.colliders,
                open_progress: SmoothInputF32::default(),
            });

        log::debug!("spawned double door: {door_entity}");
    }
}

const DOUBLE_DOOR_OPEN_SETTINGS: SmoothInputF32Settings = SmoothInputF32Settings {
    value_range: Some((0., 1.)),
    max_speed: 1.333,
    max_accel: 10.,
    max_deaccel: 10.,
};

const DOUBLE_DOOR_OPEN_DELTA: f32 = 1.677;

fn open_double_door(
    mut cmd: Commands,
    time: Singleton<SimClock>,
    mut query_door: Query<(Entity, &SwitchObserverState, &mut DoubleDoor)>,
    mut query_tf: Query<&mut Transform3>,
) {
    let dt = time.sim_dt_f32();

    for (door_entity, switch_observer, door) in query_door.iter_mut() {
        // open door if powered
        let has_power = switch_observer.as_bool();
        door.open_progress.update(
            dt,
            &DOUBLE_DOOR_OPEN_SETTINGS,
            magi::gems::SmoothInputControl::from_bool(has_power),
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
    }
}
