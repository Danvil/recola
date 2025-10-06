use crate::{
    ColliderWorld, CollidersMocca, CollisionLayer, Ray3,
    props::{door::KeyId, rift::RiftLevel},
    recola_mocca::MainCamera,
};
use candy_camera::{CameraMatrices, FirstPersonCameraController};
use candy_sky::*;
use candy_time::SimClock;
use excess::prelude::*;
use glam::{Vec2, Vec3, Vec3Swizzles};
use simplecs::prelude::*;
use std::collections::HashSet;

#[derive(Singleton)]
pub struct Player {
    pub previous_position: Vec2,

    pub eye_position: Vec3,
    pub rift_charges: HashSet<RiftLevel>,
    pub keys: HashSet<KeyId>,

    pub hours: f32,
    pub hours_target: f32,
}

/// Laser pointers with a beam which collides with objects
pub struct PlayerMocca;

impl Mocca for PlayerMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandySkyMocca>();
        deps.depends_on::<CollidersMocca>();
    }

    fn start(world: &mut World) -> Self {
        world.set_singleton(Player {
            previous_position: Vec2::ZERO,
            eye_position: Vec3::Z,
            rift_charges: HashSet::new(),
            keys: HashSet::new(),
            hours: 12.0,
            hours_target: 12.0,
        });
        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(restrict_player_movement);
        world.run(update_player_eye);
        world.run(advance_time);
    }
}

fn restrict_player_movement(
    mut player: SingletonMut<Player>,
    colliders: Singleton<ColliderWorld>,
    mut query_cam_ctrl: Query<&mut FirstPersonCameraController>,
) {
    let cam_ctrl = query_cam_ctrl
        .single_mut()
        .expect("must have FirstPersonCameraController");

    // shoot rays from eye downwards
    const TEST_RAY_RADIUS: f32 = 0.333;
    const TEST_RAY_ANGLES_DEG: [f32; 8] = [0.0_f32, 45., 90., 135., 180., 225., 270., 315.];
    let new_pos = cam_ctrl.position();
    let is_colliding = TEST_RAY_ANGLES_DEG.iter().any(|angle_deg| {
        let delta = TEST_RAY_RADIUS * Vec2::from_angle(angle_deg.to_radians());
        let origin = new_pos + Vec3::new(delta.x, delta.y, 1.7);
        let ray = Ray3::from_origin_direction(origin, -Vec3::Z).unwrap();
        colliders.raycast(&ray, None, CollisionLayer::Nav).is_some()
    });

    // If there is an obstacle set back position
    if is_colliding {
        cam_ctrl.set_position_xy(player.previous_position);
    } else {
        player.previous_position = new_pos.xy();
    }
}

fn update_player_eye(
    mut player: SingletonMut<Player>,
    query_cam: Query<&CameraMatrices, With<MainCamera>>,
) {
    let cam = query_cam.single().expect("must have MainCamera");
    player.eye_position = cam.world_t_camera().transform_point3(Vec3::ZERO);
}

const HOURS_PER_RIFT_LEVEL: f32 = 1.333;
const HOURS_ADVANCE_RATE: f32 = 0.133;

fn advance_time(
    time: Singleton<SimClock>,
    mut player: SingletonMut<Player>,
    mut day_night: SingletonMut<DayNightCycle>,
) {
    let rift_level = player
        .rift_charges
        .iter()
        .map(|lvl| lvl.0)
        .max()
        .unwrap_or(0);

    player.hours_target = 12.0 + HOURS_PER_RIFT_LEVEL * rift_level as f32;
    if player.hours < player.hours_target {
        player.hours =
            (player.hours + time.sim_dt_f32() * HOURS_ADVANCE_RATE).min(player.hours_target);
    }

    day_night.local_time = SolisticDays::from_day_hour(0, player.hours as f64);
}
