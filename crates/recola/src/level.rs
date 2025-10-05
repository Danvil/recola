use candy::{AssetInstance, AssetUid};
use candy_asset::SharedAssetResolver;
use candy_scene_tree::Transform3;
use excess::prelude::*;
use eyre::Result;
use glam::Vec3;
use magi_se::SO3;
use serde::Deserialize;
use simplecs::prelude::*;
use std::collections::HashMap;

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

pub fn spawn_level(cmd: &mut Commands, level_entity: Entity, level: &Level) {
    for inst in &level.instances {
        let entity = cmd.spawn((inst.transform(), (ChildOf, level_entity)));

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

#[derive(Component)]
pub struct CustomProperties(HashMap<String, CustomPropertiesValue>);

pub enum CustomPropertiesValue {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

impl CustomProperties {
    pub fn from_json(map: &HashMap<String, serde_json::Value>) -> Self {
        let mut out = HashMap::new();

        for (key, value) in map {
            let parsed = match value {
                serde_json::Value::Number(num) if num.is_i64() => {
                    num.as_i64().map(CustomPropertiesValue::Integer)
                }
                serde_json::Value::Number(num) if num.is_f64() => {
                    num.as_f64().map(CustomPropertiesValue::Float)
                }
                serde_json::Value::Number(_num) => {
                    todo!()
                }
                serde_json::Value::String(s) => Some(CustomPropertiesValue::String(s.clone())),
                serde_json::Value::Bool(b) => Some(CustomPropertiesValue::Bool(*b)),
                _ => {
                    todo!()
                }
            };

            if let Some(v) = parsed {
                out.insert(key.clone(), v);
            }
        }

        CustomProperties(out)
    }

    pub fn get_integer(&self, id: impl AsRef<str>) -> Option<i64> {
        match self.0.get(id.as_ref())? {
            CustomPropertiesValue::Integer(v) => Some(*v),
            _ => None,
        }
    }
}
