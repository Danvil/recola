use crate::{
    mechanics::{colliders::*, switch::*},
    player::*,
    recola_mocca::CRIMSON,
};
use candy_material::*;
use candy_prims::*;
use candy_rng::*;
use candy_scene_tree::*;
use candy_time::*;
use excess::prelude::*;
use glam::{Vec3, Vec3Swizzles};
use magi_color::colors;
use magi_se::SO3;
use simplecs::prelude::*;

#[derive(Component)]
pub struct LaserPointerAzimuth {
    pub azimuth: f32,

    pub sensitivity: f32,

    #[cfg(feature = "disco")]
    pub disco_rng_dir_cooldown: f32,
}

#[derive(Component)]
pub struct LaserPointer {
    dir: Vec3,

    beam_entity: Entity,
    exclude_collider: Entity,

    collision_point: Vec3,
    beam_length: f32,

    beam_end_entity: Entity,
}

/// Marks an entity as a target for laser beams
#[derive(Component)]
pub struct BeamDetector {
    pub latch: bool,
}

/// Set on entities with BeamHitDetector when hit by a laser beam
#[derive(Component)]
pub enum BeamHit {
    On,
    Off,
}

impl BeamHit {
    pub fn as_bool(&self) -> bool {
        match self {
            BeamHit::On => true,
            BeamHit::Off => false,
        }
    }
}

#[derive(Component)]
pub struct LaserPointerTarget {
    pub is_activated: bool,
    pub target_is_activated: bool,
    pub light_entity: Entity,
}

pub fn build_laser_pointer(cmd: &mut Commands, entity: Entity, collider_entity: Entity) {
    let beam_entity = cmd.spawn((
        Transform3::identity()
            .with_scale_xyz(MAX_BEAM_LEN, BEAM_WIDTH, BEAM_WIDTH)
            .with_translation_xyz(MAX_BEAM_LEN * 0.5, 0., 0.),
        Visibility::Visible,
        Cuboid,
        Material::Pbr(PbrMaterial::default().with_base_color(CRIMSON)),
        (ChildOf, entity),
    ));

    let beam_end_entity = cmd.spawn((
        Transform3::identity()
            .with_scale_xyz(3.0 * BEAM_WIDTH, 3.0 * BEAM_WIDTH, 3.0 * BEAM_WIDTH)
            .with_translation_xyz(MAX_BEAM_LEN, 0., 0.),
        Visibility::Visible,
        Ball,
        Material::Pbr(PbrMaterial::default().with_base_color(CRIMSON)),
        (ChildOf, entity),
    ));

    cmd.entity(collider_entity).set(CollisionRouting {
        on_raycast_entity: entity,
    });

    cmd.entity(entity).and_set(LaserPointerAzimuth {
        azimuth: 0.,
        sensitivity: 1.,

        #[cfg(feature = "disco")]
        disco_rng_dir_cooldown: 0.,
    });

    cmd.entity(entity).and_set(LaserPointer {
        dir: Vec3::Z,
        beam_entity,
        exclude_collider: collider_entity,
        collision_point: Vec3::ONE,
        beam_length: MAX_BEAM_LEN,
        beam_end_entity,
    });
}

pub fn build_laser_target(
    cmd: &mut Commands,
    name: &str,
    base_entity: Entity,
    light_entity: Entity,
) {
    cmd.entity(base_entity)
        .and_set(BeamDetector { latch: false })
        .and_set(BeamHit::Off)
        .and_set(LaserPointerTarget {
            is_activated: false,
            target_is_activated: false,
            light_entity,
        })
        .and_set(Switch { name: name.into() })
        .and_set(SwitchState::Off);
}

/// Laser pointers with a beam which collides with objects
pub struct LaserPointerMocca;

impl Mocca for LaserPointerMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyMaterialMocca>();
        deps.depends_on::<CandyPrimsMocca>();
        deps.depends_on::<CandyRngMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<PlayerMocca>();
        deps.depends_on::<SwitchMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<BeamDetector>();
        world.register_component::<BeamHit>();
        world.register_component::<LaserPointer>();
        world.register_component::<LaserPointerAzimuth>();
        world.register_component::<LaserPointerTarget>();
    }

    fn step(&mut self, world: &mut World) {
        #[cfg(feature = "disco")]
        world.run(disco_laser_pointer_azimuth);

        world.run(turn_laser_pointers);
        world.run(point_laser_pointers);
        world.run(reset_beam_hit);
        world.run(raycast_laser_beams);
        world.run(update_laser_beam_length);

        world.run(activate_laser_target);
        world.run(activate_laser_target_switch);
        world.run(set_laser_target_material);
    }
}

const MAX_BEAM_LEN: f32 = 100.;
const BEAM_WIDTH: f32 = 0.0167;
const COLLISION_HEIGHT: f32 = 4.333;
const INTERACTION_MAX_DISTANCE: f32 = 3.0;

#[cfg(feature = "disco")]
fn disco_laser_pointer_azimuth(
    time: Singleton<SimClock>,
    mut rng: SingletonMut<Rng>,
    mut query: Query<(&mut Transform3, &mut LaserPointerAzimuth)>,
) {
    let dt = time.sim_dt_f32();

    for (tf, lp) in query.iter_mut() {
        lp.disco_rng_dir_cooldown -= dt;
        if lp.disco_rng_dir_cooldown <= 0. {
            lp.disco_rng_dir_cooldown += rng.uniform(5. ..6.);
            lp.azimuth = rng.uniform_angle();
        }
    }
}

fn turn_laser_pointers(
    time: Singleton<SimClock>,
    query_input_raycast: Query<&InputRaycastController>,
    mut query_lpa: Query<&mut LaserPointerAzimuth>,
) {
    let dt = time.sim_dt_f32();
    let input_raycast = &query_input_raycast.single().unwrap();

    // Get hit entity
    let Some((hit_entity, distance)) = input_raycast.raycast_entity_and_distance() else {
        return;
    };

    // Check for turn event
    let turn_dt = if input_raycast.state().is_left_mouse_pressed {
        dt
    } else if input_raycast.state().is_right_mouse_pressed {
        -dt
    } else {
        return;
    };

    // Check we are close enough
    if distance > INTERACTION_MAX_DISTANCE {
        return;
    }

    // Get azimuth contoller
    let Some(lpa) = query_lpa.get_mut(hit_entity) else {
        return;
    };

    // Turn laser pointer
    lpa.azimuth += turn_dt * lpa.sensitivity;
}

fn point_laser_pointers(
    time: Singleton<SimClock>,
    mut query: Query<(&mut Transform3, &mut LaserPointerAzimuth, &mut LaserPointer)>,
) {
    let dt = time.sim_dt_f32();
    let point_speed = 2.0;
    let sensitivity_speed = 1.5;

    for (tf, lpa, lp) in query.iter_mut() {
        let radius = lp.collision_point.xy().length().max(0.25);
        let (asin, acos) = lpa.azimuth.sin_cos();
        let target_dir = Vec3::new(radius * acos, radius * asin, COLLISION_HEIGHT).normalize();

        lp.dir = lp.dir.lerp(target_dir, point_speed * dt).normalize();

        tf.rotation = SO3::from_to(Vec3::X, lp.dir);

        lpa.sensitivity = sensitivity_speed / radius;
    }
}

fn reset_beam_hit(mut cmd: Commands, query_detector: Query<(Entity, &BeamDetector)>) {
    for (entity, detector) in query_detector.iter() {
        if !detector.latch {
            cmd.entity(entity).set(BeamHit::Off);
        }
    }
}

fn raycast_laser_beams(
    mut cmd: Commands,
    colliders: Singleton<ColliderWorld>,
    mut query_laser_pointer: Query<(&GlobalTransform3, &mut LaserPointer)>,
    query_collision_routing: Query<&CollisionRouting>,
    query_beam_detector: Query<&BeamDetector>,
) {
    for (tf, lp) in query_laser_pointer.iter_mut() {
        let ray = Ray3::from_origin_direction(tf.translation(), tf.x_axis.into()).unwrap();

        let hit = colliders.raycast(&ray, Some(lp.exclude_collider), CollisionLayer::Laser);

        lp.beam_length = match hit {
            Some((_, len)) => len,
            None => MAX_BEAM_LEN,
        };

        lp.collision_point = tf
            .affine()
            .inverse()
            .transform_point3(ray.point(lp.beam_length));

        if let Some((hit_id, _)) = hit {
            let hit_entity = colliders[hit_id].user();
            if let Some(recv_entity) = query_collision_routing.get(hit_entity) {
                if let Some(_) = query_beam_detector.get(recv_entity.on_raycast_entity) {
                    cmd.entity(recv_entity.on_raycast_entity).set(BeamHit::On);
                }
            }
        }
    }
}

fn update_laser_beam_length(query_lp: Query<&LaserPointer>, mut query_tf: Query<&mut Transform3>) {
    for lp in query_lp.iter() {
        if let Some(tf) = query_tf.get_mut(lp.beam_entity) {
            tf.scale.x = lp.beam_length;
            tf.translation.x = 0.5 * lp.beam_length;
        }

        if let Some(tf) = query_tf.get_mut(lp.beam_end_entity) {
            tf.translation.x = lp.beam_length;
        }
    }
}

fn activate_laser_target(mut query: Query<(&mut LaserPointerTarget, &BeamHit)>) {
    for (laser_target, hit) in query.iter_mut() {
        laser_target.target_is_activated = hit.as_bool();
    }
}

fn activate_laser_target_switch(
    mut query: Query<(&BeamHit, &mut SwitchState), With<LaserPointerTarget>>,
) {
    for (hit, switch) in query.iter_mut() {
        switch.set_from_bool(hit.as_bool());
    }
}

fn set_laser_target_material(mut cmd: Commands, mut query: Query<&mut LaserPointerTarget>) {
    let mat_active = PbrMaterial::diffuse_white().with_base_color(CRIMSON);
    let mat_inactive = PbrMaterial::diffuse_white().with_base_color(colors::BLACK);

    for laser_target in query.iter_mut() {
        if laser_target.target_is_activated != laser_target.is_activated {
            laser_target.is_activated = laser_target.target_is_activated;

            let mat = if laser_target.is_activated {
                Material::Pbr(mat_active.clone())
            } else {
                Material::Pbr(mat_inactive.clone())
            };
            cmd.entity(laser_target.light_entity)
                .and_set(mat)
                .and_set(MaterialDirty);
        }
    }
}
