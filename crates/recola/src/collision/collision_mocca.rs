use crate::collision::{
    ColliderId, CollisionEntry, CollisionLayer, CollisionLayerMask, CuboidSet, Hit, PosBall3,
    PosedCuboid, Ray3,
};
use atom::prelude::*;
use candy::scene_tree::*;
use glam::Vec3;
use magi::gems::ResultLog;
use std::{
    collections::HashSet,
    ops::Index,
    sync::{Mutex, mpsc},
};

/// Marks an entity as a collider. Colliders are tracked internally in the collision world.
#[derive(Component)]
pub struct Collider(ColliderId);

/// When a collider is hit by a raycast this entity will be notified.
#[derive(Component)]
pub struct CollisionRouting {
    pub on_raycast_entity: Entity,
}

/// A set of colliders used for an entity
#[derive(Component)]
pub struct ColliderSet {
    pub collider_entities: HashSet<Entity>,
}

/// Requests a collider layer mask change
#[derive(Component)]
pub struct ChangeCollidersLayerMaskTask {
    pub mask: CollisionLayerMask,
}

/// Marks a collider as dirty which will update the corresponding collision world entry. This is
/// necessary when any of the collider properties (transform, layer, size) changes.
#[derive(Component)]
pub struct CollidersDirtyTask;

/// Global collision world which can be used for collision queries like raycasting
#[derive(Singleton)]
pub struct ColliderWorld {
    cuboids: CuboidSet,
    on_remove_rx: Mutex<mpsc::Receiver<ColliderId>>,
}

impl ColliderWorld {
    pub fn closest_exit(
        &self,
        pball: &PosBall3,
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> Option<(ColliderId, Vec3)> {
        self.cuboids.closest_exit(pball, exclude, layer)
    }

    pub fn raycast(
        &self,
        ray: &Ray3,
        radius: f32,
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> Option<Hit> {
        self.cuboids.raycast(ray, radius, exclude, layer)
    }

    pub fn closest_exit_multi_ball(
        &self,
        multi_pball: &[PosBall3],
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> Option<(ColliderId, Vec3)> {
        multi_pball
            .iter()
            .filter_map(|pball| {
                let (cid, exit) = self.cuboids.closest_exit(pball, exclude, layer)?;
                Some((cid, exit, (exit - pball.position).length()))
            })
            .min_by(|(_, _, d1), (_, _, d2)| d1.total_cmp(d2))
            .map(|(cid, exit, _)| (cid, exit))
    }

    pub fn cast_multi_ball(
        &self,
        multi_pball: &[PosBall3],
        direction: Vec3,
        exclude: Option<Entity>,
        layer: CollisionLayer,
    ) -> Option<Hit> {
        multi_pball
            .iter()
            .filter_map(|pball| {
                let hit = self.cuboids.raycast(
                    &Ray3::from_origin_normalized_direction(pball.position, direction),
                    pball.radius,
                    exclude,
                    layer,
                )?;
                Some((hit, (hit.point - pball.position).length()))
            })
            .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
            .map(|(hit, _)| hit)
    }
}

impl Index<ColliderId> for ColliderWorld {
    type Output = CollisionEntry;

    fn index(&self, id: ColliderId) -> &Self::Output {
        &self.cuboids[id]
    }
}

/// Manages colliders and provides collision queries like raycasting
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
            on_remove_tx.send(value.0).ok_log();
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

/// Tracks a collider as dirty. We need to wait one or two frames for transform updates to come
/// through.
// TODO fix this WAR
#[derive(Component, Default)]
struct DirtyCollider(usize);

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
        // FIXME We need to wait one frame for GlobalTransform3 to update. This is a WAR.
        if dirty.0 < 3 {
            dirty.0 += 1;
            continue;
        }

        // remove old collider
        if let Some(collider) = maybe_collider.as_mut() {
            collider_world.cuboids.remove(collider.0);
        }
        cmd.entity(entity).remove::<DirtyCollider>();

        // add new collider
        let cuboid = match PosedCuboid::from_unit_cube_tf(*tf.affine()) {
            Ok(cuboid) => cuboid,
            Err(err) => {
                log::error!("invalid collider for {entity}: {err:?}");
                continue;
            }
        };
        let id = collider_world.cuboids.insert(cuboid, *layer, entity);

        if let Some(collider) = maybe_collider {
            // if we would use set we would trigger a remove and a new dirty
            *collider = Collider(id);
        } else {
            cmd.entity(entity).and_set(Collider(id));
        }
    }
}
