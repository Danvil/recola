use crate::{STATIC_SETTINGS, level::*, player::*};
use atom::prelude::*;
use candy::{can::*, forge::*};
use magi::prelude::SRgbU8Color;

pub const CRIMSON: SRgbU8Color = SRgbU8Color::from_rgb(220, 20, 60);

pub struct RecolaMocca;

impl Mocca for RecolaMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<LevelMocca>();
        deps.depends_on::<PlayerMocca>();

        if STATIC_SETTINGS.enable_forge {
            deps.depends_on::<CandyForgeMocca>();
        };
    }

    fn start(_: &mut World) -> Self {
        Self
    }

    fn fini(&mut self, _world: &mut World) {
        log::info!("terminated.");
    }
}

pub struct RecolaAssetsMocca;

impl Mocca for RecolaAssetsMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyCanMocca>();
    }

    fn start(world: &mut World) -> Self {
        world.run(setup_asset_resolver);
        Self
    }
}

fn setup_asset_resolver(asset_resolver: SingletonMut<SharedAssetResolver>) {
    if asset_resolver.add_pack("recola.candy").is_err() {
        asset_resolver
            .add_pack("I:/Ikabur/recola/tmp/recola/release/recola.candy")
            .unwrap();
    }
    asset_resolver.add_prefix("assets/recola").unwrap();
    asset_resolver.add_prefix("assets/shaders").unwrap();
    asset_resolver.add_prefix("assets/candy").unwrap();
}
