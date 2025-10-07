use crate::{STATIC_SETTINGS, level::*, player::*};
use candy_forge::*;
use magi_color::SRgbU8Color;
use simplecs::prelude::*;

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
