use crate::{
    ControlMessage, GameServerMessage, LoginServerMessage, PlayerConnection,
    PlayerConnectionStatus, Throttle,
};
use gos_common::PlayerRequest;
use slab::Slab;
use std::{sync::mpsc, time::Duration};

pub struct GameServer {
    players: Slab<PlayerConnection>,
    kick: Vec<usize>,
    requests: Vec<(usize, PlayerRequest)>,
    model: Model,

    throttle: Throttle,
}

impl GameServer {
    pub fn new() -> Self {
        let throttle = Throttle::new(Duration::from_millis(1));

        Self {
            players: Slab::new(),
            kick: Vec::new(),
            requests: Vec::new(),
            model: Model::new(),
            throttle,
        }
    }

    pub fn exec_blocking(
        &mut self,
        rx_control: mpsc::Receiver<ControlMessage>,
        rx_login: mpsc::Receiver<LoginServerMessage>,
        tx_login: mpsc::Sender<GameServerMessage>,
    ) {
        loop {
            // Handle messages from login server
            loop {
                match rx_login.try_recv() {
                    Ok(LoginServerMessage::Login(player)) => {
                        log::trace!("new player: {:?}", player.user);
                        self.players.insert(player);
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("internal channel must be alive");
                    }
                }
            }

            // Handle control messages
            let mut terminated_requested = false;
            loop {
                match rx_control.try_recv() {
                    Ok(ControlMessage::Terminate) => {
                        for (id, _) in self.players.iter() {
                            self.kick.push(id);
                        }
                        terminated_requested = true;
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

            // Get messages from all players
            for (id, player) in self.players.iter_mut() {
                match player.read() {
                    PlayerConnectionStatus::Active(request) => {
                        self.requests.push((id, request));
                    }
                    PlayerConnectionStatus::Idle => {}
                    PlayerConnectionStatus::InvalidMessage | PlayerConnectionStatus::ReadFailed => {
                        self.kick.push(id);
                    }
                }
            }

            // Handle requests
            for (_id, request) in self.requests.drain(..) {
                self.model.handle_player_request(request);
            }

            // Kick players
            for id in self.kick.drain(..) {
                let player = self.players.remove(id);
                log::trace!("kicked player: {:?}", player.user);
                tx_login
                    .send(GameServerMessage::Kick(player))
                    .expect("internal channel must be alive");
            }

            if terminated_requested {
                tx_login
                    .send(GameServerMessage::Terminated)
                    .expect("internal channel must be alive");

                return;
            }

            self.throttle.throttle();
        }
    }
}

pub struct Model {
    // market: Market,
}

impl Model {
    pub fn new() -> Self {
        Self {
            // market: Market::new(),
        }
    }

    pub fn handle_player_request(&mut self, request: PlayerRequest) {
        match request {
            PlayerRequest::CreateOrder(_order) => {
                // TODO
            }
        }
    }
}
