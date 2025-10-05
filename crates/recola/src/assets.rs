use candy::{AssetLibrary, AssetUid, GltfAssetDescriptor};
use candy_asset::SharedAssetResolver;
use excess::prelude::*;
use eyre::Result;
use serde::{Deserialize, Serialize};

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
    let path = assets.resolve("assets.json")?;
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
