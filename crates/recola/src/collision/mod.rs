mod collider_set;
mod collision_mocca;
mod kernel;
mod posed_cuboid;

pub use collider_set::*;
pub use collision_mocca::*;
pub use kernel::*;
pub use posed_cuboid::*;

use glam::Vec3;
use magi::geo::Ray;

pub type Ray3 = Ray<Vec3>;

/// A 3D ball with position
pub struct PosBall3 {
    pub position: Vec3,
    pub radius: f32,
}
