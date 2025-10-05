use candy::{AssetInstance, AssetUid};
use candy_asset::SharedAssetResolver;
use candy_scene_tree::Transform3;
use excess::prelude::*;
use eyre::Result;
use glam::Vec3;
use magi_se::SO3;
use serde::Deserialize;
use simplecs::prelude::*;

#[derive(Debug, Deserialize)]
pub struct Level {
    pub instances: Vec<Instance>,
}

#[derive(Debug, Deserialize)]
pub struct Instance {
    pub name: String,
    pub asset_id: String,
    #[serde(default)]
    pub kind: Kind,
    pub location: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Collection,
    Object,
}

// If "kind" is missing, default to Object for backward compatibility.
impl Default for Kind {
    fn default() -> Self {
        Kind::Object
    }
}

pub fn spawn_level(cmd: &mut Commands, level_entity: Entity, level: &Level) {
    for inst in &level.instances {
        cmd.spawn((
            inst.transform(),
            AssetInstance(AssetUid::new(inst.asset_id.to_owned())),
            (ChildOf, level_entity),
        ));
    }
}

pub fn spawn_levels(assets: Singleton<SharedAssetResolver>, mut cmd: Commands) -> Result<()> {
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
