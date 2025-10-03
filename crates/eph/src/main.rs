mod eph_architect;
mod eph_main_window;
mod eph_mocca;

use crate::eph_mocca::EphMocca;

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
    app.load_mocca::<EphMocca>();
    app.run()
}
