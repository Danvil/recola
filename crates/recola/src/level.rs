use crate::{custom_properties::*, foundation::*};
use atom::prelude::*;
use candy::can::*;
use candy::glassworks::*;
use candy::material::*;
use candy::prims::*;
use candy::scene_tree::*;
use candy::sky::*;
use eyre::Result;
use glam::Vec3;
use magi::prelude::LinearColor;
use magi::se::SO3;
use serde::Deserialize;
use std::collections::HashMap;

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

#[derive(Debug, Deserialize)]
pub struct Level {
    pub instances: Vec<Instance>,
}

#[derive(Debug, Deserialize)]
pub struct Instance {
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

fn spawn_level(cmd: &mut Commands, level_entity: Entity, level: &Level) {
    for inst in &level.instances {
        let entity = cmd.spawn((
            Name::new(inst.name.to_owned()),
            inst.transform(),
            (ChildOf, level_entity),
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
}

fn spawn_levels(assets: Singleton<SharedAssetResolver>, mut cmd: Commands) -> Result<()> {
    let path = assets.resolve("recola.json")?;
    let world: Level = assets.parse(&path)?;

    for inst in world.instances {
        let path = assets.resolve(format!("{}.json", &inst.name))?;
        let level: Level = assets.parse(&path)?;
        let level_entity = cmd.spawn((Name::new(inst.name.to_owned()), inst.transform()));
        spawn_level(&mut cmd, level_entity, &level);
    }
    Ok(())
}
