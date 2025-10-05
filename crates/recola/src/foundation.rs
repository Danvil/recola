use crate::{
    CollidersMocca, DirtyCollider, STATIC_SETTINGS, build_laser_pointer, build_laser_target,
    load_assets,
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
    }

    fn register_components(world: &mut World) {
        world.register_component::<BlueprintApplied>();
    }

    fn start(world: &mut World) -> Self {
        let asset_resolver = world.singleton::<SharedAssetResolver>();

        asset_resolver
            .add_pack("I:/Ikabur/eph/tmp/recola/release/recola.candy")
            .unwrap();
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
    query: Query<(Entity, &AssetInstance), (With<AssetLoaded>, Without<BlueprintApplied>)>,
    children: Relation<ChildOf>,
    query_name: Query<&Name>,
) {
    for (entity, ainst) in query.iter() {
        let collider_entity = find_collider(&children, &query_name, entity);

        if let Some(collider_entity) = collider_entity {
            cmd.entity(collider_entity).set(DirtyCollider::default());

            if !STATIC_SETTINGS.show_colliders {
                cmd.entity(collider_entity).set(Visibility::Hidden)
            }
        }

        match ainst.as_str() {
            "prop-laser" => {
                let pointer =
                    find_child_by_name(&children, &query_name, entity, "pointer").unwrap();
                build_laser_pointer(&mut cmd, pointer, collider_entity);
            }
            "prop-beam_target" => {
                let target = find_child_by_name(&children, &query_name, entity, "target").unwrap();
                build_laser_target(&mut cmd, target);
            }
            _ => {}
        }

        cmd.entity(entity).set(BlueprintApplied);
    }
}

fn find_collider(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
) -> Option<Entity> {
    find_child_by_name_impl(children, query_name, entity, &|name| {
        name.ends_with("COLLIDER")
    })
}

fn find_child_by_name(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
    needle: &str,
) -> Option<Entity> {
    find_child_by_name_impl(children, query_name, entity, &|name| name == needle)
}

fn find_child_by_name_impl<F>(
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
        if let Some(out) = find_child_by_name_impl(children, query_name, child_entity, needle_f) {
            return Some(out);
        }
    }
    None
}
