use candy::{AssetInstance, AssetUid};
use candy_scene_tree::Transform3;
use excess::prelude::*;
use eyre::{Context, Result, eyre};
use glam::Vec3;
use magi_se::SO3;
use serde::Deserialize;
use simplecs::prelude::*;
use std::{fs, path::Path};

use crate::GlobalAssetPath;

#[derive(Debug, Deserialize)]
pub struct Level {
    pub instances: Vec<Instance>,
}

impl Level {
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data =
            fs::read_to_string(&path).with_context(|| eyre!("{}", path.as_ref().display()))?;
        let out = serde_json::from_str(&data)?;
        Ok(out)
    }
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

pub fn spawn_levels(asset_path: Singleton<GlobalAssetPath>, mut cmd: Commands) -> Result<()> {
    let world = Level::load_from_json(asset_path.join("recola.json"))?;
    for inst in world.instances {
        let level = Level::load_from_json(asset_path.join(&inst.name).with_extension("json"))?;
        let level_entity = cmd.spawn((Name::new(inst.name.to_owned()), inst.transform()));
        spawn_level(&mut cmd, level_entity, &level);
    }
    Ok(())
}
