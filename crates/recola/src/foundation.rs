use crate::{
    CollidersMocca, CollisionRouting, CustomProperties, DirtyCollider, STATIC_SETTINGS,
    load_assets,
    props::{
        barrier::{BarrierMocca, SpawnBarrierTask},
        door::*,
        laser_pointer::*,
        overgrowth::InitOvergrowthTask,
        rift::*,
    },
    switch::{SwitchObserver, SwitchObserverState},
};
use candy::{AssetInstance, AssetLoaded, CandyMocca};
use candy_asset::{CandyAssetMocca, SharedAssetResolver};
use candy_mesh::CandyMeshMocca;
use candy_scene_tree::{CandySceneTreeMocca, Visibility};
use excess::prelude::*;
use simplecs::prelude::*;
use std::ops::{Deref, DerefMut};

#[derive(Singleton)]
pub struct Rng(magi_rng::Rng);

impl Deref for Rng {
    type Target = magi_rng::Rng;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Rng {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Component)]
pub struct BlueprintApplied;

pub struct FoundationMocca;

impl Mocca for FoundationMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyAssetMocca>();
        deps.depends_on::<CandyMeshMocca>();
        deps.depends_on::<CandyMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<BarrierMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<BlueprintApplied>();
    }

    fn start(world: &mut World) -> Self {
        let asset_resolver = world.singleton::<SharedAssetResolver>();

        if asset_resolver.add_pack("recola.candy").is_err() {
            asset_resolver
                .add_pack("I:/Ikabur/eph/tmp/recola/release/recola.candy")
                .unwrap();
        }

        asset_resolver.add_prefix("assets/recola").unwrap();
        asset_resolver.add_prefix("assets/shaders").unwrap();

        world.set_singleton(Rng(magi_rng::Rng::from_seed(16667)));
        world.run(load_assets).unwrap();
        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(load_asset_blueprints);
    }

    fn fini(&mut self, _world: &mut World) {
        log::info!("terminated.");
    }
}

fn load_asset_blueprints(
    mut cmd: Commands,
    query: Query<
        (Entity, &AssetInstance, Option<&CustomProperties>),
        (With<AssetLoaded>, Without<BlueprintApplied>),
    >,
    children: Relation<ChildOf>,
    query_name: Query<&Name>,
) {
    for (entity, ainst, props) in query.iter() {
        let colliders = find_colliders(&children, &query_name, entity);
        for &collider_entity in &colliders {
            cmd.entity(collider_entity)
                .and_set(CollisionRouting {
                    on_raycast_entity: entity,
                })
                .and_set(DirtyCollider::default());

            if !STATIC_SETTINGS.show_colliders {
                cmd.entity(collider_entity).set(Visibility::Hidden)
            }
        }

        if let Some(props) = props {
            if let Some(switches) = props.get_string_list("switches") {
                cmd.entity(entity)
                    .and_set(SwitchObserver {
                        switches,
                        latch: false,
                    })
                    .and_set(SwitchObserverState::Inactive);
            }
        }

        match ainst.as_str() {
            "prop-laser" => {
                let pointer =
                    find_child_by_name(&children, &query_name, entity, "prop-laser.pointer")
                        .unwrap();
                build_laser_pointer(&mut cmd, pointer, colliders[0]);
            }
            "prop-beam_target" => {
                let target =
                    find_child_by_name(&children, &query_name, entity, "prop-beam_target.target")
                        .unwrap();
                let name = query_name.get(entity).unwrap().as_str();
                build_laser_target(&mut cmd, name, entity, target);
            }
            "prop-archway_3x6_door" => {
                cmd.entity(entity).set(SpawnDoorTask);
            }
            "prop-barrier_3x6" => {
                let force_field_entity = find_child_by_name(
                    &children,
                    &query_name,
                    entity,
                    "prop-barrier_3x6.force_field",
                )
                .unwrap();
                cmd.entity(entity)
                    .set(SpawnBarrierTask { force_field_entity });
            }
            "prop-rift" => {
                cmd.entity(entity).set(SpawnRiftTask);
            }
            "prop-overgrowth-1" | "prop-overgrowth-2" | "prop-overgrowth-3" => {
                let change_mat_entity = find_child(&children, &query_name, entity, |name| {
                    name.starts_with("overgrowth")
                })
                .unwrap();

                cmd.entity(entity)
                    .set(InitOvergrowthTask { change_mat_entity });
            }
            _ => {}
        }

        cmd.entity(entity).set(BlueprintApplied);
    }
}

fn find_colliders(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
) -> Vec<Entity> {
    let mut out = Vec::new();
    iter_children_by_name(children, query_name, entity, |entity, name| {
        if name.ends_with("COLLIDER") {
            out.push(entity);
        }
        false
    });
    out
}

fn find_child_by_name(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
    needle: &str,
) -> Option<Entity> {
    find_child(children, query_name, entity, |name| name == needle)
}

fn find_child<F>(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
    needle_f: F,
) -> Option<Entity>
where
    F: Fn(&str) -> bool,
{
    find_child_impl(children, query_name, entity, &needle_f)
}

fn find_child_impl<F>(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
    needle_f: &F,
) -> Option<Entity>
where
    F: Fn(&str) -> bool,
{
    for child_entity in children.iter(entity) {
        if let Some(child_name) = query_name.get(child_entity) {
            if needle_f(child_name.as_str()) {
                return Some(child_entity);
            }
        }
        if let Some(out) = find_child_impl(children, query_name, child_entity, needle_f) {
            return Some(out);
        }
    }
    None
}

fn iter_children_by_name<F>(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
    mut callback_f: F,
) -> bool
where
    F: FnMut(Entity, &str) -> bool,
{
    iter_children_by_name_impl(children, query_name, entity, &mut callback_f)
}

fn iter_children_by_name_impl<F>(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
    callback_f: &mut F,
) -> bool
where
    F: FnMut(Entity, &str) -> bool,
{
    for child_entity in children.iter(entity) {
        if let Some(child_name) = query_name.get(child_entity) {
            if callback_f(child_entity, child_name.as_str()) {
                return true;
            }
        }
        if iter_children_by_name_impl(children, query_name, child_entity, callback_f) {
            return true;
        }
    }
    false
}
