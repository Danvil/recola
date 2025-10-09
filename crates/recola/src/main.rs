pub mod custom_properties;
pub mod foundation;
pub mod level;
pub mod mechanics;
pub mod player;
pub mod props;

mod recola_mocca;
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

    let mut app = candy::glassworks::App::new();
    app.load_mocca::<RecolaMocca>();
    app.run()
}
