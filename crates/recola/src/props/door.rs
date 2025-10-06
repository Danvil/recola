use crate::{
    CollidersMocca, CollisionLayerMask, CustomProperties, DirtyCollider, FoundationMocca, Player,
    PlayerMocca, recola_mocca::InputRaycastController,
};
use candy::CandyMocca;
use candy_scene_tree::{CandySceneTreeMocca, Transform3};
use candy_time::{CandyTimeMocca, SimClock};
use excess::prelude::*;
use eyre::{Result, eyre};
use simplecs::prelude::*;

#[derive(Component)]
pub struct SpawnDoorTask {
    pub collider_entity: Entity,
}

#[derive(Component, Debug, Clone)]
pub struct Door {
    collider_entity: Entity,
    lower_progress: f32,
    progress_changed: bool,
    is_lowered: bool,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyId(pub i64);

/// Laser pointers with a beam which collides with objects
pub struct DoorMocca;

impl Mocca for DoorMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<FoundationMocca>();
        deps.depends_on::<PlayerMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<Door>();
        world.register_component::<KeyId>();
        world.register_component::<SpawnDoorTask>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(spawn_door);
        world.run(lower_door_interaction);
        world.run(lower_door);
    }
}

const DOOR_INTERACTION_DISTANCE: f32 = 5.;
const DOOR_LOWER_SPEED: f32 = 1.333;
const DOOR_LOWER_MAX: f32 = 3.933;

fn spawn_door(
    mut cmd: Commands,
    query_open_door_task: Query<(Entity, &SpawnDoorTask)>,
    query_props: Query<&CustomProperties>,
) {
    for (door_entity, task) in query_open_door_task.iter() {
        cmd.entity(door_entity).remove::<SpawnDoorTask>();

        let key_id = match get_key_id(&query_props, door_entity) {
            Ok(key_id) => key_id,
            Err(err) => {
                log::error!("door without key_id: {err:?}");
                continue;
            }
        };

        cmd.entity(door_entity)
            .and_set(Door {
                collider_entity: task.collider_entity,
                lower_progress: 0.,
                progress_changed: false,
                is_lowered: false,
            })
            .and_set(key_id);
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

fn lower_door_interaction(
    time: Singleton<SimClock>,
    player: Singleton<Player>,
    query_input_raycast: Query<&InputRaycastController>,
    mut query_door: Query<(&mut Door, &KeyId)>,
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
    if distance > DOOR_INTERACTION_DISTANCE {
        return;
    }

    // Get door
    let Some((door, key)) = query_door.get_mut(hit_entity) else {
        return;
    };

    // Operate door
    if !door.is_lowered {
        if player.keys.contains(key) {
            door.lower_progress += DOOR_LOWER_SPEED * dt;
            if door.lower_progress >= DOOR_LOWER_MAX {
                door.lower_progress = DOOR_LOWER_MAX;
                door.is_lowered = true;
            }
            door.progress_changed = true;
        } else {
            log::debug!("missing key: {key:?}");
        }
    }
}

fn lower_door(
    mut cmd: Commands,
    mut query_door: Query<(Entity, &mut Transform3, &mut Door)>,
    mut query_collider: Query<&mut CollisionLayerMask>,
) {
    for (door_entity, tf, door) in query_door.iter_mut() {
        if door.progress_changed {
            tf.translation.z = -door.lower_progress;

            if door.is_lowered {
                log::debug!("door {door_entity} lowered");
                query_collider.get_mut(door.collider_entity).unwrap().nav = false;
                cmd.entity(door.collider_entity)
                    .set(DirtyCollider::default());
            }
        }
        door.progress_changed = false;
    }
}
