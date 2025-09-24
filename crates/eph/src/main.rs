mod eph_mocca;

use crate::eph_mocca::EphMocca;

fn main() -> eyre::Result<()> {
    env_logger::init();
    let mut app = candy::App::new();
    app.load_mocca::<EphMocca>();
    app.run()
}
