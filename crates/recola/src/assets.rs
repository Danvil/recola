use candy::{AssetLibrary, AssetUid, GltfAssetDescriptor};
use excess::prelude::*;
use eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};

#[derive(Singleton)]
pub struct GlobalAssetPath(pub PathBuf);

impl AsRef<Path> for GlobalAssetPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Deref for GlobalAssetPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize)]
struct AssetCollection {
    assets: Vec<AssetEntry>,
}

impl AssetCollection {
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data =
            fs::read_to_string(&path).with_context(|| eyre!("{}", path.as_ref().display()))?;
        let out = serde_json::from_str(&data)?;
        Ok(out)
    }
}

#[derive(Serialize, Deserialize)]
struct AssetEntry {
    name: String,
    file: String,
    scene: String,
    node: String,
}

pub fn load_assets(
    asset_path: Singleton<GlobalAssetPath>,
    mut asli: SingletonMut<AssetLibrary>,
) -> Result<()> {
    let coll = AssetCollection::load_from_json(asset_path.as_ref().join("assets.json"))?;
    for entry in coll.assets {
        asli.load_gltf(
            &AssetUid::new(entry.name),
            GltfAssetDescriptor {
                path: asset_path.join(entry.file),
                scene: Some(entry.scene),
                node: Some(entry.node),
            },
        );
    }
    Ok(())
}
