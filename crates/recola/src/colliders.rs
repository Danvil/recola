use candy_scene_tree::{CandySceneTreeMocca, GlobalTransform3};
use excess::prelude::*;
use glam::{Affine3A, Vec3};
use magi_geo::Ray;
use simplecs::prelude::*;
use slab::Slab;
use std::{
    ops::Index,
    sync::{Mutex, mpsc},
};

pub type Ray3 = Ray<Vec3>;

#[derive(Component)]
pub struct Collider(ColliderId);

#[derive(Component, Default)]
pub struct DirtyCollider(usize);

#[derive(Component)]
pub struct CollisionRouting {
    pub on_raycast_entity: Entity,
}

#[derive(Singleton)]
pub struct ColliderWorld {
    pub cuboids: CuboidSet,
    pub on_remove_rx: Mutex<mpsc::Receiver<ColliderId>>,
}

impl ColliderWorld {
    pub fn raycast(&self, ray: &Ray3, exclude: Option<Entity>) -> Option<(ColliderId, f32)> {
        self.cuboids.raycast(ray, exclude)
    }
}

impl Index<ColliderId> for ColliderWorld {
    type Output = PosedCuboid;

    fn index(&self, id: ColliderId) -> &Self::Output {
        &self.cuboids[id]
    }
}

pub struct CuboidSet {
    cuboids: Slab<PosedCuboid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColliderId(usize);

impl CuboidSet {
    pub fn new() -> Self {
        Self {
            cuboids: Slab::new(),
        }
    }

    pub fn insert(&mut self, ref_t_cuboid: Affine3A, half_size: Vec3, user: Entity) -> ColliderId {
        let cuboid_t_ref = ref_t_cuboid.inverse();

        let idx = self.cuboids.insert(PosedCuboid {
            user,
            ref_t_cuboid,
            cuboid_t_ref,
            half_size,
        });

        ColliderId(idx)
    }

    pub fn remove(&mut self, id: ColliderId) {
        self.cuboids.remove(id.0);
    }

    pub fn raycast(&self, ray: &Ray3, exclude: Option<Entity>) -> Option<(ColliderId, f32)> {
        let out = self
            .cuboids
            .iter()
            .filter(|(_, cub)| Some(cub.user) != exclude)
            .filter_map(|(idx, cub)| cub.raycast(ray).map(|lam| (idx, lam)))
            .min_by_key(|(_, lam)| (lam * 10000.) as i64)
            .map(|(idx, lam)| (ColliderId(idx), lam));

        out
    }
}

impl Default for CuboidSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<ColliderId> for CuboidSet {
    type Output = PosedCuboid;

    fn index(&self, id: ColliderId) -> &Self::Output {
        &self.cuboids[id.0]
    }
}

pub struct PosedCuboid {
    user: Entity,
    ref_t_cuboid: Affine3A,
    cuboid_t_ref: Affine3A,
    half_size: Vec3,
}

impl PosedCuboid {
    pub fn raycast(&self, ray: &Ray3) -> Option<f32> {
        aabb_raycast(self.half_size, ray.transform(&self.cuboid_t_ref))
    }

    pub fn user(&self) -> Entity {
        self.user
    }
}

/// Intersects a ray with an axis-aligned box centered at the origin with half-extents `half_size`.
/// Returns the nearest non-negative hit distance if it exists.
pub fn aabb_raycast(half_size: Vec3, ray: Ray3) -> Option<f32> {
    // Reciprocal; inf handles zero components correctly for slab tests.
    let inv_dir = Vec3::ONE / ray.direction();

    // Parametric distances to the slabs on each axis.
    let t1 = (-half_size - ray.origin) * inv_dir;
    let t2 = (half_size - ray.origin) * inv_dir;

    // Entry is the maximum of the per-axis minima; exit is the minimum of the per-axis maxima.
    let t_min_v = t1.min(t2);
    let t_max_v = t1.max(t2);

    let t_enter = t_min_v.x.max(t_min_v.y).max(t_min_v.z);
    let t_exit = t_max_v.x.min(t_max_v.y).min(t_max_v.z);

    // No hit if the slabs miss or the box is entirely behind the ray.
    if t_enter > t_exit || t_exit < 0.0 {
        return None;
    }

    // If starting outside, return entry; if starting inside (t_enter < 0), return exit.
    Some(if t_enter >= 0.0 { t_enter } else { t_exit })
}

/// Manages colliders and provides raycasting
pub struct CollidersMocca {
    on_remove_hook_id: OnRemoveHookId,
}

impl Mocca for CollidersMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandySceneTreeMocca>();
    }

    fn start(world: &mut World) -> Self {
        let (on_remove_tx, on_remove_rx) = mpsc::channel();

        world.set_singleton(ColliderWorld {
            cuboids: CuboidSet::new(),
            on_remove_rx: Mutex::new(on_remove_rx),
        });

        let on_remove_hook_id = world.insert_on_remove_hook(move |_key, value: &Collider| {
            on_remove_tx.send(value.0).unwrap();
        });

        Self { on_remove_hook_id }
    }

    fn register_components(world: &mut World) {
        world.register_component::<Collider>();
        world.register_component::<DirtyCollider>();
        world.register_component::<CollisionRouting>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(update_colliders);
        world.run(create_colliders);
    }

    fn fini(&mut self, world: &mut World) {
        world
            .remove_on_remove_hook(Collider::id(), self.on_remove_hook_id)
            .unwrap();
    }
}

fn update_colliders(
    mut collider_world: SingletonMut<ColliderWorld>,
    mut cmd: Commands,
    query: Query<(Entity, &Collider), With<DirtyCollider>>,
) {
    // Add new colliders
    for (entity, collider) in query.iter() {
        collider_world.cuboids.remove(collider.0);
        cmd.entity(entity).remove::<Collider>();
    }

    // Handle removed colliders
    let ids: Vec<_> = {
        let rx = collider_world.on_remove_rx.lock().unwrap();
        rx.try_iter().collect()
    };
    for id in ids {
        println!("Collider removed: {id:?}");
        collider_world.cuboids.remove(id);
    }
}

fn create_colliders(
    mut collider_world: SingletonMut<ColliderWorld>,
    mut cmd: Commands,
    mut query: Query<
        (Entity, &GlobalTransform3, &mut DirtyCollider),
        (Without<Collider>, With<DirtyCollider>),
    >,
) {
    for (entity, tf, dirty) in query.iter_mut() {
        // TODO we need to wait one frame for GlobalTransform3 to update ..
        if dirty.0 < 10 {
            dirty.0 += 1;
            continue;
        }

        let half_size = Vec3::ONE;

        let id = collider_world
            .cuboids
            .insert(*tf.affine(), half_size, entity);

        cmd.entity(entity)
            .and_set(Collider(id))
            .remove::<DirtyCollider>();
    }
}
