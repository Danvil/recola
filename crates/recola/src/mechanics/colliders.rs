use atom::prelude::*;
use candy::scene_tree::*;
use glam::{Affine3A, Vec3};
use magi::geo::{Aabb, Ray};
use slab::Slab;
use std::{
    collections::HashSet,
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

/// A set of colliders
#[derive(Component)]
pub struct ColliderSet {
    pub collider_entities: HashSet<Entity>,
}

#[derive(Singleton)]
pub struct ColliderWorld {
    pub cuboids: CuboidSet,
    pub on_remove_rx: Mutex<mpsc::Receiver<ColliderId>>,
}

#[derive(Component)]
pub struct ChangeCollidersLayerMaskTask {
    pub mask: CollisionLayerMask,
}

#[derive(Component)]
pub struct CollidersDirtyTask;

impl ColliderWorld {
    pub fn raycast(
        &self,
        ray: &Ray3,
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> Option<(ColliderId, f32)> {
        self.cuboids.raycast(ray, exclude, layer)
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

    pub fn insert(
        &mut self,
        ref_t_cuboid: Affine3A,
        half_size: Vec3,
        layer: CollisionLayerMask,
        user: Entity,
    ) -> ColliderId {
        let cuboid_t_ref = ref_t_cuboid.inverse();

        let idx = self.cuboids.insert(PosedCuboid {
            ref_t_cuboid,
            cuboid_t_ref,
            half_size,
            layer_mask: layer,
            user,
        });

        ColliderId(idx)
    }

    pub fn remove(&mut self, id: ColliderId) {
        if self.cuboids.contains(id.0) {
            self.cuboids.remove(id.0);
        }
    }

    pub fn raycast(
        &self,
        ray: &Ray3,
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> Option<(ColliderId, f32)> {
        let out = self
            .cuboids
            .iter()
            .filter(|(_, cub)| cub.layer_mask.matches(layer))
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

pub struct PosedCuboid {
    ref_t_cuboid: Affine3A,
    cuboid_t_ref: Affine3A,
    half_size: Vec3,
    layer_mask: CollisionLayerMask,
    user: Entity,
}

impl PosedCuboid {
    pub fn raycast(&self, ray: &Ray3) -> Option<f32> {
        aabb_raycast(self.half_size, ray.transform(&self.cuboid_t_ref))
    }

    pub fn user(&self) -> Entity {
        self.user
    }

    pub fn half_size(&self) -> Vec3 {
        self.half_size
    }

    pub fn ref_t_cuboid(&self) -> &Affine3A {
        &self.ref_t_cuboid
    }

    pub fn aabb(&self) -> Aabb<Vec3> {
        Aabb::from_points(
            [
                Vec3::new(1., 1., 1.),
                Vec3::new(1., 1., -1.),
                Vec3::new(1., -1., 1.),
                Vec3::new(1., -1., -1.),
                Vec3::new(-1., 1., 1.),
                Vec3::new(-1., 1., -1.),
                Vec3::new(-1., -1., 1.),
                Vec3::new(-1., -1., -1.),
            ]
            .iter()
            .map(|p| self.ref_t_cuboid.transform_point3(p * self.half_size)),
        )
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
        world.register_component::<ChangeCollidersLayerMaskTask>();
        world.register_component::<Collider>();
        world.register_component::<ColliderSet>();
        world.register_component::<CollisionLayerMask>();
        world.register_component::<CollisionRouting>();
        world.register_component::<DirtyCollider>();
        world.register_component::<CollidersDirtyTask>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(change_collider_layer_mask_tasks);
        world.run(colliders_dirty_tasks);
        world.run(remove_colliders_of_despawned_entities);
        world.run(update_dirty_colliders);
    }

    fn fini(&mut self, world: &mut World) {
        world
            .remove_on_remove_hook(Collider::id(), self.on_remove_hook_id)
            .unwrap();
    }
}

fn change_collider_layer_mask_tasks(
    mut cmd: Commands,
    query_tasks: Query<(Entity, &ColliderSet, &ChangeCollidersLayerMaskTask)>,
    mut query_collider: Query<&mut CollisionLayerMask>,
) {
    for (entity, collider_set, task) in query_tasks.iter() {
        for &collider_entity in &collider_set.collider_entities {
            if let Some(mask) = query_collider.get_mut(collider_entity) {
                *mask = task.mask;
                cmd.entity(collider_entity)
                    .and_set(DirtyCollider::default());
            }
        }
        cmd.entity(entity).remove::<ChangeCollidersLayerMaskTask>();
    }
}

fn colliders_dirty_tasks(
    mut cmd: Commands,
    query_tasks: Query<(Entity, &ColliderSet), With<CollidersDirtyTask>>,
    query_dirty: Query<&DirtyCollider>,
) {
    for (entity, collider_set) in query_tasks.iter() {
        for &collider_entity in &collider_set.collider_entities {
            if query_dirty.get(collider_entity).is_none() {
                cmd.entity(collider_entity)
                    .and_set(DirtyCollider::default());
            }
        }
        cmd.entity(entity).remove::<CollidersDirtyTask>();
    }
}

fn remove_colliders_of_despawned_entities(mut collider_world: SingletonMut<ColliderWorld>) {
    // Handle removed colliders
    let ids: Vec<_> = {
        let rx = collider_world.on_remove_rx.lock().unwrap();
        rx.try_iter().collect()
    };
    for id in ids {
        log::debug!("Collider removed: {id:?}");
        collider_world.cuboids.remove(id);
    }
}

fn update_dirty_colliders(
    mut collider_world: SingletonMut<ColliderWorld>,
    mut cmd: Commands,
    mut query: Query<
        (
            Entity,
            &GlobalTransform3,
            Option<&mut Collider>,
            &mut DirtyCollider,
            &CollisionLayerMask,
        ),
        With<DirtyCollider>,
    >,
) {
    for (entity, tf, mut maybe_collider, dirty, layer) in query.iter_mut() {
        // TODO we need to wait one frame for GlobalTransform3 to update ..
        if dirty.0 < 3 {
            dirty.0 += 1;
            continue;
        }

        // remove old collider
        if let Some(collider) = maybe_collider.as_mut() {
            collider_world.cuboids.remove(collider.0);
        }

        // TODO currently we assume that collider geometry is a unit cube
        let half_size = Vec3::ONE;

        // add new collider
        let id = collider_world
            .cuboids
            .insert(*tf.affine(), half_size, *layer, entity);

        if let Some(collider) = maybe_collider {
            // if we would use set we would trigger a remove and a new dirty
            *collider = Collider(id);
        } else {
            cmd.entity(entity).and_set(Collider(id));
        }

        cmd.entity(entity).remove::<DirtyCollider>();
    }
}
