use crate::{
    ColliderWorld, CollidersMocca, CollisionRouting, FoundationMocca, Ray3, Rng,
    recola_mocca::{CRIMSON, InputRaycastController, MainCamera},
};
use candy::{CandyMocca, MaterialDirty};
use candy_camera::CameraMatrices;
use candy_mesh::{Ball, Cuboid};
use candy_scene_tree::{CandySceneTreeMocca, GlobalTransform3, Transform3, Visibility};
use candy_time::{CandyTimeMocca, SimClock};
use candy_utils::{Material, PbrMaterial};
use excess::prelude::*;
use glam::{Vec3, Vec3Swizzles};
use magi_color::{SRgbU8Color, colors};
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
    exclude_collider: Option<Entity>,

    collision_point: Vec3,
    beam_length: f32,

    beam_end_entity: Entity,
}

#[derive(Component)]
pub struct LaserPointerTarget {
    pub is_activated: bool,
    pub target_is_activated: bool,

    #[cfg(feature = "disco")]
    pub debug_disco_counter: usize,
}

pub fn build_laser_pointer(cmd: &mut Commands, entity: Entity, collider_entity: Option<Entity>) {
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

    if let Some(collider_entity) = collider_entity {
        cmd.entity(collider_entity).set(CollisionRouting {
            on_raycast_entity: entity,
        });
    }

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

pub fn build_laser_target(cmd: &mut Commands, entity: Entity) {
    cmd.entity(entity)
        .and_set(LaserPointerTarget {
            is_activated: false,
            target_is_activated: false,

            #[cfg(feature = "disco")]
            debug_disco_counter: 0,
        })
        .and_set(Material::Pbr(PbrMaterial::default()));
}

/// Laser pointers with a beam which collides with objects
pub struct LaserPointerMocca;

impl Mocca for LaserPointerMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<FoundationMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<LaserPointer>();
        world.register_component::<LaserPointerAzimuth>();
        world.register_component::<LaserPointerTarget>();
    }

    fn step(&mut self, world: &mut World) {
        #[cfg(feature = "disco")]
        world.run(disco_laser_pointer_azimuth);

        world.run(turn_laser_pointers);
        world.run(point_laser_pointers);
        world.run(collide_laser_beams);
        world.run(update_laser_beam_length);
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

fn collide_laser_beams(
    colliders: Singleton<ColliderWorld>,
    mut query: Query<(&GlobalTransform3, &mut LaserPointer)>,
) {
    for (tf, lp) in query.iter_mut() {
        let ray = Ray3::from_origin_direction(tf.translation(), tf.x_axis.into()).unwrap();

        lp.beam_length = match colliders.raycast(&ray, lp.exclude_collider) {
            Some((_, len)) => len,
            None => MAX_BEAM_LEN,
        };

        lp.collision_point = tf
            .affine()
            .inverse()
            .transform_point3(ray.point(lp.beam_length));
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

fn set_laser_target_material(
    mut cmd: Commands,
    mut query: Query<(Entity, &mut Material, &mut LaserPointerTarget)>,
) {
    let mat_active = PbrMaterial::diffuse_white().with_base_color(CRIMSON);
    let mat_inactive = PbrMaterial::diffuse_white().with_base_color(colors::BLACK);

    for (entity, mat, laser_target) in query.iter_mut() {
        if laser_target.target_is_activated != laser_target.is_activated {
            laser_target.is_activated = laser_target.target_is_activated;

            if laser_target.is_activated {
                *mat = Material::Pbr(mat_active.clone());
            } else {
                *mat = Material::Pbr(mat_inactive.clone());
            }

            cmd.entity(entity).set(MaterialDirty);
        }

        #[cfg(feature = "disco")]
        {
            laser_target.debug_disco_counter += 1;
            if laser_target.debug_disco_counter > 100 {
                laser_target.target_is_activated = !laser_target.is_activated;
                laser_target.debug_disco_counter = 0;
            }
        }
    }
}
