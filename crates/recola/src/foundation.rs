use crate::{
    STATIC_SETTINGS,
    collision::*,
    custom_properties::*,
    mechanics::switch::*,
    props::{barrier::*, door::*, laser_pointer::*, overgrowth::*, rift::*},
    recola_mocca::{CRIMSON, RecolaAssetsMocca},
};
use atom::prelude::*;
use candy::{
    audio::{AudioEmitterBody, SpatialAudioEmitter},
    can::*,
    glassworks::*,
    scene_tree::*,
};
use eyre::Result;
use magi::{color::colors, geo::Aabb};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Component)]
pub struct BlueprintApplied;

pub struct FoundationMocca;

impl Mocca for FoundationMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<BarrierMocca>();
        deps.depends_on::<CandyCanMocca>();
        deps.depends_on::<CandyGlassworksMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<CustomPropertiesMocca>();
        deps.depends_on::<DoorMocca>();
        deps.depends_on::<LaserPointerMocca>();
        deps.depends_on::<OvergrowthMocca>();
        deps.depends_on::<RecolaAssetsMocca>();
        deps.depends_on::<RiftMocca>();
        deps.depends_on::<SwitchMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<BlueprintApplied>();
    }

    fn start(world: &mut World) -> Self {
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

#[derive(Serialize, Deserialize)]
struct AssetCollection {
    assets: Vec<AssetEntry>,
}

#[derive(Serialize, Deserialize)]
struct AssetEntry {
    name: String,
    file: String,
    scene: String,
    node: String,
}

pub fn load_assets(
    assets: Singleton<SharedAssetResolver>,
    mut asli: SingletonMut<AssetLibrary>,
) -> Result<()> {
    let path = assets.resolve("props.json")?;
    let coll: AssetCollection = assets.parse(&path)?;

    for entry in coll.assets {
        let path = assets.resolve(&entry.file)?;
        asli.load_gltf(
            &AssetUid::new(entry.name),
            GltfAssetDescriptor {
                path,
                scene: Some(entry.scene),
                node: Some(entry.node),
            },
        );
    }
    Ok(())
}

fn load_asset_blueprints(
    mut cmd: Commands,
    query: Query<
        (Entity, &AssetInstance, Option<&CustomProperties>),
        (With<AssetLoaded>, Without<BlueprintApplied>),
    >,
    children: Relation<ChildOf>,
    query_tf: Query<&Transform3>,
    query_name: Query<&Name>,
) {
    for (entity, ainst, props) in query.iter() {
        // Setup colliders
        let colliders = find_colliders(&children, &query_name, entity);
        for &(collider_entity, collision_layer_mask) in &colliders {
            cmd.entity(collider_entity)
                .and_set(CollisionRouting {
                    on_raycast_entity: entity,
                })
                .and_set(collision_layer_mask);

            if !STATIC_SETTINGS.show_colliders {
                cmd.entity(collider_entity).set(Visibility::Hidden)
            }
        }

        cmd.entity(entity)
            .and_set(ColliderSet {
                collider_entities: HashSet::from_iter(colliders.iter().map(|(e, _)| *e)),
            })
            .and_set(CollidersDirtyTask);

        // Setup audio emitter
        let audio_emitter_entity = find_child(&children, &query_name, entity, |name| {
            name.ends_with("AUDIO_EMITTER")
        });
        if let Some(audio_emitter_entity) = audio_emitter_entity {
            let tf = query_tf.get(audio_emitter_entity).unwrap();

            if tf.rotation.to_axis_angle().1.abs() > 1.0f32.to_radians() {
                log::warn!("Audio emitter rotation is not aligned with the world axes");
            }

            // Assuming the collider is a unit cube
            let aabb = Aabb::from_centroid_size(tf.translation, tf.scale);

            cmd.entity(entity).and_set(SpatialAudioEmitter {
                body: AudioEmitterBody::Aabb(aabb),
            });

            if !STATIC_SETTINGS.show_audio_emitters {
                cmd.entity(audio_emitter_entity).set(Visibility::Hidden)
            }
        };

        // Setup switch
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

                cmd.entity(pointer).set(SpawnLaserPointer {
                    audio_entity: entity,
                    collider_entity: colliders[0].0,
                });
            }
            "prop-beam_target" | "prop-barrier_switch" => {
                let switch_id = query_name.get(entity).unwrap().as_str().to_owned();

                let indicator_entity = find_child(&children, &query_name, entity, |name| {
                    name.ends_with("indicator")
                })
                .unwrap();

                let active_color = match ainst.as_str() {
                    "prop-beam_target" => CRIMSON,
                    "prop-barrier_switch" => PROP_BARRIER_SWITCH_INDICATOR_COLOR,
                    _ => unreachable!(),
                };

                cmd.entity(entity).set(SpawnLaserTarget {
                    switch_id,
                    indicator_entity,
                    activate_emission_color: active_color.to_linear() * 5.0,
                    inactivate_emission_color: colors::BLACK.into(),
                });
            }
            "prop-archway_3x6_door" => {
                let relief_entity = find_child(&children, &query_name, entity, |name| {
                    name.ends_with("relief")
                })
                .unwrap();

                cmd.entity(entity).set(SpawnLevelGateTask { relief_entity });
            }
            "prop-gate_door" => {
                let left_leaf = find_child(&children, &query_name, entity, |name| {
                    name.ends_with("left")
                })
                .unwrap();

                let right_leaf = find_child(&children, &query_name, entity, |name| {
                    name.ends_with("right")
                })
                .unwrap();

                let left_collider = find_child(&children, &query_name, entity, |name| {
                    name.ends_with("left-COLLIDER")
                })
                .unwrap();

                let right_collider = find_child(&children, &query_name, entity, |name| {
                    name.ends_with("right-COLLIDER")
                })
                .unwrap();

                cmd.entity(entity).set(SpawnDoubleDoorTask {
                    leafes: [(left_leaf, 6.0), (right_leaf, 0.0)],
                    colliders: [(left_collider, 4.0), (right_collider, 2.0)],
                });
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
            "prop-overgrowth-1"
            | "prop-overgrowth-2"
            | "prop-overgrowth-3"
            | "prop-overgrowth_3x3_1" => {
                let change_mat_entity = find_child(&children, &query_name, entity, |name| {
                    name.ends_with("burn")
                })
                .unwrap();

                cmd.entity(entity)
                    .set(SpawnOvergrowthTask { change_mat_entity });
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
) -> Vec<(Entity, CollisionLayerMask)> {
    let mut out = Vec::new();
    iter_children_by_name(children, query_name, entity, |entity, name| {
        if name.ends_with("COLLIDER") {
            out.push((entity, CollisionLayerMask::all()));
        } else if name.ends_with("COLLIDER_INTERACT") {
            out.push((entity, CollisionLayerMask::only_interact()));
        } else if name.ends_with("COLLIDER_NAV") {
            out.push((entity, CollisionLayerMask::only_nav()));
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
