use crate::{
    CollidersMocca, CollisionRouting, DirtyCollider, FoundationMocca, Player, PlayerMocca, Rng,
    recola_mocca::InputRaycastController,
};
use candy::{AssetInstance, AssetUid, CandyMocca};
use candy_scene_tree::{CandySceneTreeMocca, Transform3, Visibility};
use candy_time::{CandyTimeMocca, SimClock};
use excess::prelude::*;
use glam::Vec3;
use simplecs::prelude::*;

pub fn spawn_rift(mut spawn: impl Spawn, rng: &mut Rng, transform: Transform3) -> Entity {
    let jitter = 0.1;

    let rift_entity = spawn.spawn((
        Name::from_str("rift"),
        transform,
        RiftConsume {
            is_consumed: false,
            charge: 0.,
        },
        Visibility::Visible,
    ));

    let _rift_collider_entity = spawn.spawn((
        Transform3::identity(),
        Visibility::Hidden,
        DirtyCollider::default(),
        CollisionRouting {
            on_raycast_entity: rift_entity,
        },
        (ChildOf, rift_entity),
    ));

    for _ in 0..20 {
        let anchor = 2.0 * (rng.unit_vec3() - 0.5) * jitter;

        spawn.spawn((
            Name::from_str("rift"),
            RiftJitter {
                anchor,
                target: anchor,
                speed: 0.16667,
                cooldown: 0.2 * rng.unit_f32(),
            },
            Transform3::from_translation(anchor),
            AssetInstance(AssetUid::new("prop-rift")),
            (ChildOf, rift_entity),
        ));
    }

    rift_entity
}

/// Laser pointers with a beam which collides with objects
pub struct RiftMocca;

impl Mocca for RiftMocca {
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
        world.register_component::<RiftConsume>();
        world.register_component::<RiftJitter>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(rift_jitter);
        world.run(charge_rift_interaction);
        world.run(consume_rift);
    }
}

const INTERACTION_MAX_DISTANCE: f32 = 3.0;

#[derive(Component)]
struct RiftJitter {
    anchor: Vec3,
    target: Vec3,
    speed: f32,
    cooldown: f32,
}

#[derive(Component)]
struct RiftConsume {
    is_consumed: bool,
    charge: f32,
}

fn rift_jitter(
    time: Singleton<SimClock>,
    mut rng: SingletonMut<Rng>,
    mut query: Query<(&mut RiftJitter, &mut Transform3)>,
) {
    let dt = time.sim_dt_f32();

    let jitter = Vec3::new(0.133, 0.133, 0.333);

    for (jit, tf) in query.iter_mut() {
        jit.cooldown -= dt;
        if jit.cooldown <= 0. {
            jit.cooldown += rng.uniform(1. ..3.);
            let delta = 2.0 * (rng.unit_vec3() - 0.5);
            jit.target = jit.anchor + delta * jitter;
        }
        let dir = jit.target - tf.translation;
        tf.translation += dir * jit.speed * dt;
    }
}

fn charge_rift_interaction(
    time: Singleton<SimClock>,
    query_input_raycast: Query<&InputRaycastController>,
    mut query_rift_consume: Query<&mut RiftConsume>,
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
    if distance > INTERACTION_MAX_DISTANCE {
        return;
    }

    // Get rift consume
    let Some(rift_consume) = query_rift_consume.get_mut(hit_entity) else {
        return;
    };

    // Turn laser pointer
    rift_consume.charge += RIFT_CHARGE_RATE * dt;
}

const RIFT_CHARGE_TO_CONSUME: f32 = 3.33;
const RIFT_CHARGE_RATE: f32 = 1.3;
const RIFT_DECHARGE_RATE: f32 = 0.3;

fn consume_rift(
    time: Singleton<SimClock>,
    mut player: SingletonMut<Player>,
    mut query_rift_consume: Query<(Entity, &mut RiftConsume)>,
    mut query_tf_vis: Query<(&mut Transform3, &mut Visibility)>,
) {
    let dt = time.sim_dt_f32();

    for (entity, rift_consume) in query_rift_consume.iter_mut() {
        let (tf, vis) = query_tf_vis.get_mut(entity).unwrap();

        if rift_consume.is_consumed {
            continue;
        }

        if rift_consume.charge >= RIFT_CHARGE_TO_CONSUME {
            rift_consume.is_consumed = true;
            player.rift_charges += 1;

            *vis = Visibility::Hidden;
        }

        rift_consume.charge = (rift_consume.charge - RIFT_DECHARGE_RATE * dt).max(0.);

        let q = (1.0 - rift_consume.charge / RIFT_CHARGE_TO_CONSUME).max(0.);
        let scale = q;
        tf.scale = scale * Vec3::ONE;
    }
}
