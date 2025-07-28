use crate::{
    ConnectServerMessage, Connection, ConnectionStatus, ControlMessage, GameServerMessage,
    LoginServerMessage, PlayerConnection, Throttle,
};
use core::net::Ipv4Addr;
use gos_common::{ClientRequest, requests};
use slab::Slab;
use std::{sync::mpsc, time::Duration};
use steamworks as sw;

/// Handles connection authentication using Steamworks
pub struct LoginServer {
    connections: Slab<Connection>,
    steam_server: sw::Server,

    invalid_connections: Vec<usize>,
    login_requests: Vec<(usize, requests::Login)>,
    logout_requests: Vec<PlayerConnection>,

    throttle: Throttle,
}

pub struct LoginServerConfig {
    pub ip: Ipv4Addr,
    pub port: u16,
}

const SERVER_VERSION: &str = "0.1.0";

impl LoginServer {
    pub fn new(config: LoginServerConfig) -> eyre::Result<Self> {
        let (steam_server, _) = sw::Server::init(
            config.ip,
            config.port - 1,
            config.port,
            sw::ServerMode::AuthenticationAndSecure,
            SERVER_VERSION,
        )?;

        steam_server.set_map_name("dystopia");
        steam_server.set_server_name("universe");
        steam_server.set_max_players(100);

        let throttle = Throttle::new(Duration::from_millis(1));

        Ok(Self {
            connections: Slab::new(),
            steam_server,
            invalid_connections: Vec::new(),
            login_requests: Vec::new(),
            logout_requests: Vec::new(),
            throttle,
        })
    }

    pub fn exec_blocking(
        &mut self,
        rx_connect: mpsc::Receiver<ConnectServerMessage>,
        rx_game: mpsc::Receiver<GameServerMessage>,
        tx_login: mpsc::Sender<LoginServerMessage>,
        tx_ctrl: mpsc::Sender<ControlMessage>,
    ) {
        loop {
            // Handle messages from connection server
            loop {
                match rx_connect.try_recv() {
                    Ok(ConnectServerMessage::Incoming(stream)) => {
                        self.connections.insert(Connection::new(stream));
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("internal channel must be alive");
                    }
                }
            }

            // Handle messages from game server
            let mut game_server_terminated = false;
            loop {
                match rx_game.try_recv() {
                    Ok(GameServerMessage::Kick(con)) => {
                        self.logout_requests.push(con);
                    }
                    Ok(GameServerMessage::Terminated) => {
                        game_server_terminated = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("internal channel must be alive");
                    }
                }
            }

            // Handle logout requests
            for con in self.logout_requests.drain(..) {
                log::trace!("logout request: user={:?}", con.user);
                self.steam_server.end_authentication_session(con.user);
            }

            if game_server_terminated {
                tx_ctrl
                    .send(ControlMessage::Terminate)
                    .expect("internal channel must be alive");

                break;
            }

            // Check connections for login requests
            for (id, con) in self.connections.iter_mut() {
                match con.read() {
                    ConnectionStatus::Active(request) => match request {
                        ClientRequest::Login(login) => {
                            self.login_requests.push((id, login));
                        }
                        _ => {
                            self.invalid_connections.push(id);
                        }
                    },
                    ConnectionStatus::Idle => {}
                    ConnectionStatus::InvalidMessage | ConnectionStatus::ReadFailed => {
                        self.invalid_connections.push(id);
                    }
                }

                if con.age() > Duration::from_millis(1000) {
                    self.invalid_connections.push(id);
                }
            }

            // Handle login requests
            for (id, login) in self.login_requests.drain(..) {
                log::trace!("login request: user={:?}", login.steam_user);
                let user = sw::SteamId::from_raw(login.steam_user);

                match self
                    .steam_server
                    .begin_authentication_session(user, &login.steam_auth)
                {
                    Ok(()) => {
                        let connection = self.connections.remove(id);

                        tx_login
                            .send(LoginServerMessage::Login(PlayerConnection {
                                connection,
                                user,
                            }))
                            .expect("internal channel must be alive");
                    }
                    Err(err) => {
                        log::error!("authentication failed: {err:?}");
                        self.invalid_connections.push(id);
                    }
                }
            }

            // Reject invalid connections
            for id in self.invalid_connections.drain(..) {
                self.connections.remove(id);
            }

            self.throttle.throttle();
        }
    }
}
