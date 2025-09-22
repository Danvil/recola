mod eph_mocca;

use crate::eph_mocca::EphMocca;

fn main() -> eyre::Result<()> {
    let mut app = candy::App::new();
    app.load_mocca::<EphMocca>();
    app.run()
}
