use crate::collision::{PosBall3, PosedCuboid, Ray3};
use atom::prelude::*;
use glam::Vec3;
use magi::geo::Aabb;
use slab::Slab;
use std::ops::Index;

pub struct CuboidSet {
    entries: Slab<CollisionEntry>,
}

impl CuboidSet {
    pub fn new() -> Self {
        Self {
            entries: Slab::new(),
        }
    }

    pub fn insert(
        &mut self,
        cuboid: PosedCuboid,
        layer: CollisionLayerMask,
        user: Entity,
    ) -> ColliderId {
        let idx = self.entries.insert(CollisionEntry {
            cuboid,
            layer_mask: layer,
            user,
        });
        ColliderId(idx)
    }

    pub fn remove(&mut self, id: ColliderId) {
        if self.entries.contains(id.0) {
            self.entries.remove(id.0);
        }
    }

    pub fn iter_filtered(
        &self,
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> impl Iterator<Item = (usize, &PosedCuboid)> {
        self.entries
            .iter()
            .filter(move |(_, entry)| entry.layer_mask.matches(layer))
            .filter(move |(_, entry)| Some(entry.user) != exclude)
            .map(|(idx, entry)| (idx, &entry.cuboid))
    }

    pub fn signed_distance_pos_ball(
        &self,
        exclude: Option<Entity>,
        layer: CollisionLayer,
        pball: &PosBall3,
    ) -> Option<f32> {
        self.iter_filtered(exclude, layer)
            .map(|(_, cub)| cub.signed_distance_pos_ball(pball))
            .min_by(|d1, d2| d1.total_cmp(d2))
    }

    /// Returns the closest point to exit if the point is inside a collider. If the point is
    /// inside multiple colliders the closest collider is picked. In this case the exit point
    /// might still lie inside another collider.
    pub fn closest_exit(
        &self,
        pball: &PosBall3,
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> Option<(ColliderId, Vec3)> {
        self.iter_filtered(exclude, layer)
            .filter_map(|(idx, cub)| {
                cub.closest_exit(pball)
                    .map(|exit| (idx, exit, (pball.position - exit).length()))
            })
            .min_by(|(_, _, d1), (_, _, d2)| d1.total_cmp(d2))
            .map(|(idx, exit, _)| (ColliderId(idx), exit))
    }

    /// Intersects cubes with a ray. The ray radius can be 0 (point ray) or greater zero for
    /// a "sphere cast".
    pub fn raycast(
        &self,
        ray: &Ray3,
        radius: f32,
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> Option<Hit> {
        self.iter_filtered(exclude, layer)
            .filter_map(|(idx, cub)| cub.raycast(ray, radius).map(|(lam, n)| (idx, lam, n)))
            .min_by(|(_, d1, _), (_, d2, _)| d1.total_cmp(d2))
            .map(|(idx, distance, normal)| Hit {
                id: ColliderId(idx),
                distance,
                point: ray.point(distance),
                normal,
            })
    }
}

impl Default for CuboidSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<ColliderId> for CuboidSet {
    type Output = CollisionEntry;

    fn index(&self, id: ColliderId) -> &Self::Output {
        &self.entries[id.0]
    }
}

pub struct CollisionEntry {
    pub cuboid: PosedCuboid,
    pub layer_mask: CollisionLayerMask,
    pub user: Entity,
}

impl CollisionEntry {
    pub fn aabb(&self) -> Aabb<Vec3> {
        self.cuboid.aabb()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColliderId(usize);

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub id: ColliderId,
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionLayer {
    Laser,
    Interact,
    Nav,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionLayerMask {
    pub laser: bool,
    pub interact: bool,
    pub nav: bool,
}

impl CollisionLayerMask {
    pub fn all() -> Self {
        Self {
            laser: true,
            interact: true,
            nav: true,
        }
    }

    pub fn none() -> Self {
        Self {
            laser: false,
            interact: false,
            nav: false,
        }
    }

    pub fn only_nav() -> Self {
        Self {
            laser: false,
            interact: false,
            nav: true,
        }
    }

    pub fn only_interact() -> Self {
        Self {
            laser: false,
            interact: true,
            nav: false,
        }
    }

    pub fn matches(&self, layer: CollisionLayer) -> bool {
        match layer {
            CollisionLayer::Laser => self.laser,
            CollisionLayer::Interact => self.interact,
            CollisionLayer::Nav => self.nav,
        }
    }
}
