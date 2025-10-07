use crate::{
    custom_properties::*,
    mechanics::{colliders::*, switch::*},
    player::*,
    props::door::KeyId,
    recola_mocca::CRIMSON,
};
use candy::{AssetInstance, AssetUid};
use candy_mesh::*;
use candy_rng::*;
use candy_scene_tree::*;
use candy_time::*;
use candy_utils::{Material, PbrMaterial};
use excess::prelude::*;
use glam::Vec3;
use simplecs::prelude::*;

#[derive(Component)]
pub struct SpawnRiftTask;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RiftLevel(pub i64);

/// Laser pointers with a beam which collides with objects
pub struct RiftMocca;

impl Mocca for RiftMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyMeshMocca>();
        deps.depends_on::<CandyRngMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<PlayerMocca>();
        deps.depends_on::<SwitchMocca>();
        deps.depends_on::<CustomPropertiesMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<OpenRiftTask>();
        world.register_component::<Rift>();
        world.register_component::<RiftConsume>();
        world.register_component::<RiftConsumeParticle>();
        world.register_component::<RiftLevel>();
        world.register_component::<RiftJitter>();
        world.register_component::<RiftShardInflate>();
        world.register_component::<SpawnRiftTask>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(spawn_rift);
        world.run(activate_rift);
        world.run(open_rift);
        world.run(inflate_rift_shards);
        world.run(rift_jitter);
        world.run(charge_rift_interaction);
        world.run(consume_rift);
        world.run(spawn_rift_consume_particles);
        world.run(animate_rift_consume_particles);
    }
}

const INTERACTION_MAX_DISTANCE: f32 = 3.0;

#[derive(Component)]
struct Rift;

#[derive(Component)]
struct RiftConsume {
    is_consumed: bool,
    charge: f32,
    particle_charge: f32,
}

#[derive(Component)]
struct RiftJitter {
    anchor: Vec3,
    target: Vec3,
    speed: f32,
    cooldown: f32,
}

#[derive(Component)]
struct RiftShardInflate {
    progress: f32,
}

fn spawn_rift(
    mut cmd: Commands,
    mut query_open_rift_task: Query<
        (
            Entity,
            &mut Transform3,
            &CustomProperties,
            Option<&mut SwitchObserver>,
        ),
        With<SpawnRiftTask>,
    >,
) {
    for (rift_entity, tf, props, switch_observer) in query_open_rift_task.iter_mut() {
        cmd.entity(rift_entity).remove::<SpawnRiftTask>();

        let Some(rift_id) = props.get_integer("rift_id") else {
            log::error!("SpawnRiftTask CustomProperties without 'rift_id'");
            continue;
        };

        cmd.entity(rift_entity)
            .and_set(Rift)
            .and_set(RiftLevel(rift_id));

        if let Some(switch_observer) = switch_observer {
            switch_observer.latch = true;
        } else {
            cmd.entity(rift_entity).and_set(OpenRiftTask);
        }
    }
}

fn activate_rift(
    mut cmd: Commands,
    query: Query<(Entity, &SwitchObserverState), (With<Rift>, Without<RiftConsume>)>,
) {
    for (rift_entity, activation_state) in query.iter() {
        if activation_state.as_bool() {
            cmd.entity(rift_entity).and_set(OpenRiftTask);
        }
    }
}

#[derive(Component)]
struct OpenRiftTask;

const RIFT_SHARDS_INITIAL_POS_JITTER: f32 = 0.1;

fn open_rift(
    mut cmd: Commands,
    mut rng: SingletonMut<Rng>,
    query: Query<Entity, With<OpenRiftTask>>,
) {
    for rift_entity in query.iter() {
        cmd.entity(rift_entity).remove::<OpenRiftTask>();

        for _ in 0..20 {
            let anchor = 2.0 * (rng.unit_vec3() - 0.5) * RIFT_SHARDS_INITIAL_POS_JITTER;

            cmd.spawn((
                Name::from_str("rift"),
                RiftJitter {
                    anchor,
                    target: anchor,
                    speed: 0.16667,
                    cooldown: 0.2 * rng.unit_f32(),
                },
                Transform3::from_translation(anchor),
                RiftShardInflate { progress: 0. },
                AssetInstance(AssetUid::new("prop-rift_schimmer")),
                (ChildOf, rift_entity),
            ));

            cmd.entity(rift_entity).and_set(RiftConsume {
                is_consumed: false,
                charge: 0.,
                particle_charge: 0.,
            });
        }
    }
}

const RIFT_SHARD_INFLATE_MAX_PROGRESS: f32 = 0.7;

fn inflate_rift_shards(
    mut cmd: Commands,
    time: Singleton<SimClock>,
    mut query: Query<(Entity, &mut RiftShardInflate, &mut Transform3)>,
) {
    let dt = time.sim_dt_f32();

    for (entity, infl, tf) in query.iter_mut() {
        infl.progress += dt;
        let q = infl.progress / RIFT_SHARD_INFLATE_MAX_PROGRESS;
        tf.scale = Vec3::splat(q);
        if q >= 1. {
            cmd.entity(entity).remove::<RiftShardInflate>();
        }
    }
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

    // Consume rift
    if !rift_consume.is_consumed {
        rift_consume.charge += (RIFT_CHARGE_RATE + RIFT_DECHARGE_RATE) * dt;
        rift_consume.particle_charge += dt;
    }
}

const RIFT_CHARGE_TO_CONSUME: f32 = 3.33;
const RIFT_CHARGE_RATE: f32 = 1.33;
const RIFT_DECHARGE_RATE: f32 = 0.333;

fn consume_rift(
    mut cmd: Commands,
    time: Singleton<SimClock>,
    mut player: SingletonMut<Player>,
    mut query_rift_consume: Query<(Entity, &mut RiftConsume, &RiftLevel)>,
    mut query_tf: Query<&mut Transform3>,
) {
    let dt = time.sim_dt_f32();

    for (entity, rift_consume, rift_id) in query_rift_consume.iter_mut() {
        let tf = query_tf.get_mut(entity).unwrap();

        if rift_consume.is_consumed {
            continue;
        }

        if rift_consume.charge >= RIFT_CHARGE_TO_CONSUME {
            rift_consume.is_consumed = true;
            player.rift_charges.insert(*rift_id);
            let key = KeyId(rift_id.0);
            log::debug!("acquired key: {key:?}");
            player.keys.insert(key);

            cmd.entity(entity).set(Visibility::Hidden);
        }

        rift_consume.charge = (rift_consume.charge - RIFT_DECHARGE_RATE * dt).max(0.);

        let q = (1.0 - rift_consume.charge / RIFT_CHARGE_TO_CONSUME).max(0.);
        let scale = q;
        tf.scale = scale * Vec3::ONE;
    }
}

#[derive(Component)]
struct RiftConsumeParticle {
    age: f32,
    size: f32,
    target_offset: Vec3,
}

const RIFT_CONSUME_PARTICLE_SPAWN_RATE: f32 = 0.0333;
const RIFT_CONSUME_PARTICLE_SPAWN_POSITION_VAR: f32 = 0.333;
const RIFT_CONSUME_PARTICLE_TARGET_VAR: f32 = 0.333;
const RIFT_CONSUME_PARTICLE_SIZE: f32 = 0.0667;
const RIFT_CONSUME_PARTICLE_SPEED: f32 = 5.0;
const RIFT_CONSUME_PARTICLE_TIME_TO_MAX_SIZE: f32 = 0.133;

fn spawn_rift_consume_particles(
    mut cmd: Commands,
    mut rng: SingletonMut<Rng>,
    mut query: Query<(&GlobalTransform3, &mut RiftConsume)>,
) {
    for (tf, rift_consume) in query.iter_mut() {
        if rift_consume.is_consumed {
            continue;
        }

        if rift_consume.particle_charge >= RIFT_CONSUME_PARTICLE_SPAWN_RATE {
            rift_consume.particle_charge -= RIFT_CONSUME_PARTICLE_SPAWN_RATE;

            cmd.spawn((
                RiftConsumeParticle {
                    age: 0.,
                    size: 0.,
                    target_offset: RIFT_CONSUME_PARTICLE_TARGET_VAR * rng.sphere_point(),
                },
                Cuboid,
                Material::Pbr(PbrMaterial::default().with_base_color(CRIMSON)),
                Visibility::Visible,
                Transform3::identity()
                    .with_translation(
                        tf.translation()
                            + RIFT_CONSUME_PARTICLE_SPAWN_POSITION_VAR * rng.nunit_vec3(),
                    )
                    .with_rotation(rng.uniform_so3())
                    .with_scale_uniform(0.),
            ));
        }
    }
}

fn animate_rift_consume_particles(
    mut cmd: Commands,
    time: Singleton<SimClock>,
    player: Singleton<Player>,
    mut query: Query<(Entity, &mut RiftConsumeParticle, &mut Transform3)>,
) {
    let dt = time.sim_dt_f32();
    let step = dt * RIFT_CONSUME_PARTICLE_SPEED;

    for (entity, particle, tf) in query.iter_mut() {
        particle.age += dt;
        particle.size = RIFT_CONSUME_PARTICLE_SIZE
            * (particle.age / RIFT_CONSUME_PARTICLE_TIME_TO_MAX_SIZE).min(1.);
        tf.scale = Vec3::splat(particle.size);

        let delta = player.eye_position + particle.target_offset - tf.translation;
        if delta.length() < 5.0 * step {
            cmd.despawn(entity);
        } else {
            tf.translation += delta.normalize() * step;
        }
    }
}
