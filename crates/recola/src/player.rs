use crate::{
    props::{door::KeyId, rift::RiftLevel},
    recola_mocca::MainCamera,
};
use candy_camera::CameraMatrices;
use candy_sky::*;
use candy_time::SimClock;
use excess::prelude::*;
use glam::Vec3;
use simplecs::prelude::*;
use std::collections::HashSet;

#[derive(Singleton)]
pub struct Player {
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
    }

    fn start(world: &mut World) -> Self {
        world.set_singleton(Player {
            eye_position: Vec3::Z,
            rift_charges: HashSet::new(),
            keys: HashSet::new(),
            hours: 12.0,
            hours_target: 12.0,
        });
        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(update_player_position);
        world.run(advance_time);
    }
}

fn update_player_position(
    mut player: SingletonMut<Player>,
    query: Query<&CameraMatrices, With<MainCamera>>,
) {
    player.eye_position = query
        .single()
        .expect("must have MainCamera")
        .world_t_camera()
        .transform_point3(Vec3::ZERO);
}

const HOURS_PER_RIFT_LEVEL: f32 = 1.0;
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
