mod assets;
mod colliders;
mod door;
mod foundation;
mod laser_pointer;
mod level;
mod nav;
mod player;
pub mod props;
mod recola_mocca;
mod rift;

pub use assets::*;
pub use colliders::*;
pub use door::*;
pub use foundation::*;
pub use laser_pointer::*;
pub use level::*;
pub use nav::*;
pub use player::*;
pub use rift::*;

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
