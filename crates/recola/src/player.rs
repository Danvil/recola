use crate::{
    props::{door::KeyId, rift::RiftId},
    recola_mocca::MainCamera,
};
use candy_camera::CameraMatrices;
use excess::prelude::*;
use glam::Vec3;
use simplecs::prelude::*;
use std::collections::HashSet;

#[derive(Singleton)]
pub struct Player {
    pub eye_position: Vec3,
    pub rift_charges: HashSet<RiftId>,
    pub keys: HashSet<KeyId>,
}

/// Laser pointers with a beam which collides with objects
pub struct PlayerMocca;

impl Mocca for PlayerMocca {
    fn start(world: &mut World) -> Self {
        world.set_singleton(Player {
            eye_position: Vec3::Z,
            rift_charges: HashSet::new(),
            keys: HashSet::new(),
        });
        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(update_player_position);
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
