mod assets;
mod level;
mod recola_mocca;

use crate::recola_mocca::RecolaMocca;
pub use assets::*;
pub use level::*;

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
