mod assets;
mod colliders;
mod foundation;
mod laser_pointer;
mod level;
mod recola_mocca;

pub use assets::*;
pub use colliders::*;
pub use foundation::*;
pub use laser_pointer::*;
pub use level::*;

use crate::recola_mocca::RecolaMocca;

pub struct StaticSettings {
    enable_forge: bool,
    show_colliders: bool,
}

pub const STATIC_SETTINGS: StaticSettings = StaticSettings {
    enable_forge: true,
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
