mod assets;
mod colliders;
mod custom_properties;
mod foundation;
mod level;
mod mechanics;
mod nav;
mod player;
pub mod props;
mod recola_mocca;

pub use assets::*;
pub use colliders::*;
pub use custom_properties::*;
pub use foundation::*;
pub use level::*;
pub use mechanics::*;
pub use player::*;

use crate::recola_mocca::RecolaMocca;

pub struct StaticSettings {
    enable_forge: bool,
    show_colliders: bool,
}

pub const STATIC_SETTINGS: StaticSettings = StaticSettings {
    enable_forge: false,
    show_colliders: false,
};

fn main() -> eyre::Result<()> {
    env_logger::init();

    profiling::register_thread!("main");

    #[cfg(feature = "profile-with-tracy")]
    {
        log::info!("starting tracy client");
        tracy_client::Client::start();
    }

    profiling::scope!("main");

    let mut app = candy::App::new();
    app.load_mocca::<RecolaMocca>();
    app.run()
}
