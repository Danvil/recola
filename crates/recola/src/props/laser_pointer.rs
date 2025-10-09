use crate::{
    mechanics::{colliders::*, switch::*},
    player::*,
};
use atom::prelude::*;
use candy::material::*;
use candy::prims::*;
use candy::rng::*;
use candy::scene_tree::*;
use candy::time::*;
use glam::{Vec3, Vec3Swizzles};
use magi::{color::*, se::SO3};

pub const NEON_BLUE: SRgbU8Color = SRgbU8Color::from_rgb(20, 160, 220);

/// Spawns a laser pointer on an entity
#[derive(Component)]
pub struct SpawnLaserPointer {
    /// Collider entity of the laser pointer
    pub collider_entity: Entity,
}

/// Spawns a laser target on an entity
#[derive(Component)]
pub struct SpawnLaserTarget {
    /// The switch ID
    pub switch_id: String,

    /// When the target is hit by a laser beam the material of this entity will be changed
    pub indicator_entity: Entity,

    /// The emission color of the indicator when activated by a laser beam
    pub activate_emission_color: LinearColor,

    /// The emission color of the indicator when not activated by a laser beam
    pub inactivate_emission_color: LinearColor,
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
        world.register_component::<SpawnLaserPointer>();
        world.register_component::<SpawnLaserTarget>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(spawn_laser_pointer);
        world.run(spawn_laser_target);

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

#[derive(Component)]
struct LaserPointerAzimuth {
    azimuth: f32,

    sensitivity: f32,

    #[cfg(feature = "disco")]
    disco_rng_dir_cooldown: f32,
}

#[derive(Component)]
struct LaserPointer {
    dir: Vec3,

    beam_entity: Entity,
    exclude_collider: Entity,

    collision_point: Vec3,
    beam_length: f32,

    beam_end_entity: Entity,
}

#[derive(Component)]
struct LaserPointerTarget {
    is_activated: bool,
    target_is_activated: bool,
    light_entity: Entity,
    activate_emission_color: LinearColor,
    inactivate_emission_color: LinearColor,
}

const MAX_BEAM_LEN: f32 = 100.;
const BEAM_WIDTH: f32 = 0.0167;
const COLLISION_HEIGHT: f32 = 4.80;
const INTERACTION_MAX_DISTANCE: f32 = 3.0;
const POINTER_EMIT_HEIGHT: f32 = 1.333;

fn spawn_laser_pointer(mut cmd: Commands, query: Query<(Entity, &SpawnLaserPointer)>) {
    for (entity, spec) in query.iter() {
        let beam_entity = cmd.spawn((
            Transform3::identity()
                .with_scale_xyz(MAX_BEAM_LEN, BEAM_WIDTH, BEAM_WIDTH)
                .with_translation_xyz(MAX_BEAM_LEN * 0.5, 0., 0.),
            DynamicTransform,
            Visibility::Visible,
            Cuboid,
            Material::Pbr(
                PbrMaterial::default()
                    .with_base_color(NEON_BLUE)
                    .with_emission(NEON_BLUE.to_linear() * 15.0),
            ),
            (ChildOf, entity),
        ));

        let beam_end_entity = cmd.spawn((
            Transform3::identity()
                .with_scale_xyz(3.0 * BEAM_WIDTH, 3.0 * BEAM_WIDTH, 3.0 * BEAM_WIDTH)
                .with_translation_xyz(MAX_BEAM_LEN, 0., 0.),
            DynamicTransform,
            Visibility::Visible,
            Ball,
            Material::Pbr(
                PbrMaterial::default()
                    .with_base_color(NEON_BLUE)
                    .with_emission(NEON_BLUE.to_linear() * 20.0),
            ),
            (ChildOf, entity),
        ));

        cmd.entity(entity)
            .and_remove::<SpawnLaserPointer>()
            .and_set(LaserPointerAzimuth {
                azimuth: 0.,
                sensitivity: 1.,

                #[cfg(feature = "disco")]
                disco_rng_dir_cooldown: 0.,
            })
            .and_set(DynamicTransform)
            .and_set(LaserPointer {
                dir: Vec3::Z,
                beam_entity,
                exclude_collider: spec.collider_entity,
                collision_point: Vec3::ONE,
                beam_length: MAX_BEAM_LEN,
                beam_end_entity,
            });

        cmd.entity(spec.collider_entity).set(CollisionRouting {
            on_raycast_entity: entity,
        });
    }
}

fn spawn_laser_target(mut cmd: Commands, query: Query<(Entity, &SpawnLaserTarget)>) {
    for (entity, spec) in query.iter() {
        cmd.entity(entity)
            .and_remove::<SpawnLaserTarget>()
            .and_set(BeamDetector { latch: false })
            .and_set(BeamHit::Off)
            .and_set(LaserPointerTarget {
                is_activated: false,
                target_is_activated: false,
                light_entity: spec.indicator_entity,
                activate_emission_color: spec.activate_emission_color,
                inactivate_emission_color: spec.inactivate_emission_color,
            })
            .and_set(Switch {
                name: spec.switch_id.clone(),
            })
            .and_set(SwitchState::Off);
    }
}

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
        let target_dir = Vec3::new(
            radius * acos,
            radius * asin,
            COLLISION_HEIGHT - POINTER_EMIT_HEIGHT,
        )
        .normalize();

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
    for laser_target in query.iter_mut() {
        if laser_target.target_is_activated != laser_target.is_activated {
            laser_target.is_activated = laser_target.target_is_activated;

            let mat = if laser_target.is_activated {
                Material::Pbr(
                    PbrMaterial::diffuse_white()
                        .with_base_color(colors::BLACK)
                        .with_emission(laser_target.activate_emission_color),
                )
            } else {
                Material::Pbr(
                    PbrMaterial::diffuse_white()
                        .with_base_color(colors::BLACK)
                        .with_emission(laser_target.inactivate_emission_color),
                )
            };
            cmd.entity(laser_target.light_entity)
                .and_set(mat)
                .and_set(MaterialDirty);
        }
    }
}
