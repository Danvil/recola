use crate::{custom_properties::*, foundation::*};
use atom::prelude::*;
use candy::{can::*, glassworks::*, material::*, prims::*, scene_tree::*, sky::*};
use eyre::Result;
use glam::Vec3;
use magi::{prelude::LinearColor, se::SO3};
use serde::Deserialize;
use std::collections::HashMap;

/// List of loaded levels
#[derive(Singleton, Default)]
pub struct LevelSummary {
    pub pos: Vec<Vec3>,
}

/// Loads the world of Recola
pub struct LevelMocca;

impl Mocca for LevelMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyCanMocca>();
        deps.depends_on::<CandyGlassworksMocca>();
        deps.depends_on::<CandyMaterialMocca>();
        deps.depends_on::<CandyPrimsMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandySkyMocca>();
        deps.depends_on::<CustomPropertiesMocca>();
        deps.depends_on::<FoundationMocca>();
    }

    fn start(world: &mut World) -> Self {
        world.run(setup_sky);
        world.run(spawn_terrain);
        world.run(spawn_levels).unwrap();
        Self
    }
}

fn setup_sky(mut sky: SingletonMut<SkyModel>, mut day_night: SingletonMut<DayNightCycle>) {
    sky.set_sun_raw_radiance(15.0);
    sky.set_moon_raw_radiance(0.35);
    day_night.speed_factor = 0.;
    day_night.local_time = SolisticDays::from_day_hour(0, 12.0);
}

fn spawn_terrain(mut cmd: Commands) {
    const GROUND_PLANE_SIZE: f32 = 1024.;

    cmd.spawn((
        Name::from_str("ground plane"),
        Transform3::identity()
            .with_translation(Vec3::new(0.0, 0.0, -0.55))
            .with_scale(Vec3::new(GROUND_PLANE_SIZE, GROUND_PLANE_SIZE, 1.)),
        Cuboid,
        HierarchyDirty,
        Visibility::Visible,
        Material::Pbr(PbrMaterial::diffuse(LinearColor::from_rgb(
            0.10, 0.10, 0.10,
        ))),
    ));
}

#[derive(Debug, Deserialize)]
struct Level {
    pub instances: Vec<Instance>,
}

#[derive(Debug, Deserialize)]
struct Instance {
    pub name: String,
    pub asset_id: Option<String>,
    pub location: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],

    /// Custom properties on the instance object.
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Instance {
    pub fn transform(&self) -> Transform3 {
        Transform3 {
            translation: Vec3::from(self.location),
            rotation: SO3::from_xyzw_array(self.rotation),
            scale: Vec3::from(self.scale),
        }
    }
}

fn spawn_levels(assets: Singleton<SharedAssetResolver>, mut cmd: Commands) -> Result<()> {
    let path = assets.resolve("recola.json")?;
    let world: Level = assets.parse(&path)?;

    let world_entity = cmd.spawn((Name::new("world"), Transform3::identity()));

    let mut level_pos_by_name = Vec::new();

    for inst in world.instances {
        if let Ok(path) = assets.resolve(format!("{}.json", &inst.name)) {
            let level: Level = assets.parse(&path)?;
            let tf = inst.transform();
            level_pos_by_name.push((inst.name.clone(), tf.translation));
            spawn_level(&mut cmd, inst.name, tf, level);
        } else {
            spawn_instance(&mut cmd, world_entity, inst);
        }
    }

    level_pos_by_name.sort_by_key(|(name, _)| name.clone());

    cmd.set_singleton(LevelSummary {
        pos: level_pos_by_name.into_iter().map(|(_, pos)| pos).collect(),
    });

    Ok(())
}

fn spawn_level(cmd: &mut Commands, name: String, tf: Transform3, level: Level) {
    let level_entity = cmd.spawn((Name::new(name), tf));
    for inst in level.instances {
        spawn_instance(cmd, level_entity, inst);
    }
}

fn spawn_instance(cmd: &mut Commands, parent: Entity, inst: Instance) {
    let entity = cmd.spawn((
        Name::new(inst.name.to_owned()),
        inst.transform(),
        (ChildOf, parent),
    ));

    if let Some(asset_id) = inst.asset_id.as_ref() {
        let ainst = AssetInstance(AssetUid::new(asset_id.to_owned()));
        cmd.entity(entity).set(ainst);
    }

    if !inst.custom.is_empty() {
        let props = CustomProperties::from_json(&inst.custom);
        cmd.entity(entity).set(props);
    }
}
