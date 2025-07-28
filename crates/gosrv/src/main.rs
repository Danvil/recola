use std::{sync::mpsc, thread};

mod connect_server;
mod connection;
mod game_server;
mod internal_messages;
mod login_server;
mod throttle;

pub use connect_server::*;
pub use connection::*;
pub use game_server::*;
pub use internal_messages::*;
pub use login_server::*;
pub use throttle::*;

fn main() -> eyre::Result<()> {
    env_logger::init();

    log::info!("GOS server");

    let mut connect = ConnectServer::new("127.0.0.1:9000".to_string())?;
    let mut login = LoginServer::new(LoginServerConfig {
        ip: "127.0.0.0".parse()?,
        port: 54321,
    })?;
    let mut game = GameServer::new();

    let (tx_control_game, rx_control_game) = mpsc::channel();
    let (tx_control_login, rx_control_login) = mpsc::channel();
    let (tx_connect, rx_connect) = mpsc::channel();
    let (tx_login, rx_login) = mpsc::channel();
    let (tx_game, rx_game) = mpsc::channel();

    let thread_1 = thread::spawn(move || connect.exec_blocking(rx_control_login, tx_connect));
    let thread_2 =
        thread::spawn(move || login.exec_blocking(rx_connect, rx_game, tx_login, tx_control_login));
    let thread_3 = thread::spawn(move || game.exec_blocking(rx_control_game, rx_login, tx_game));

    ctrlc::set_handler(move || {
        log::warn!("received ctrl+c - terminating");
        tx_control_game.send(ControlMessage::Terminate).ok();
    })
    .expect("Error setting Ctrl-C handler");

    log::info!("server is running");

    thread_3.join().ok();
    log::info!("game server terminated");
    thread_2.join().ok();
    log::info!("login server terminated");
    thread_1.join().ok();
    log::info!("connection server terminated");

    log::info!("server terminated successfully");

    Ok(())
}
