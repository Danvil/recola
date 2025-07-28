use gos_common::{ClientRequest, PlayerRequest};
use std::{
    io::Read,
    net::TcpStream,
    time::{Duration, Instant},
};
use steamworks as sw;

pub struct Connection {
    connection_time: Instant,
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            connection_time: Instant::now(),
            stream,
            buffer: vec![0; 256],
        }
    }

    pub fn age(&self) -> Duration {
        Instant::now() - self.connection_time
    }

    pub fn read(&mut self) -> ConnectionStatus {
        match self.stream.read(&mut self.buffer) {
            Ok(n) => {
                if n == 0 {
                    // connection terminated
                    ConnectionStatus::ReadFailed
                } else {
                    match bincode::serde::decode_from_slice(
                        &self.buffer[0..n],
                        bincode::config::standard(),
                    ) {
                        Ok((msg, _)) => ConnectionStatus::Active(msg),
                        Err(_err) => ConnectionStatus::InvalidMessage,
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => ConnectionStatus::Idle,
            Err(_err) => ConnectionStatus::ReadFailed,
        }
    }
}

pub enum ConnectionStatus {
    Active(ClientRequest),
    Idle,
    ReadFailed,
    InvalidMessage,
}

pub struct PlayerConnection {
    pub connection: Connection,
    pub user: sw::SteamId,
}

pub enum PlayerConnectionStatus {
    Active(PlayerRequest),
    Idle,
    ReadFailed,
    InvalidMessage,
}

impl PlayerConnection {
    pub fn new(connection: Connection, user: sw::SteamId) -> Self {
        Self { connection, user }
    }

    pub fn read(&mut self) -> PlayerConnectionStatus {
        match self.connection.read() {
            ConnectionStatus::Active(ClientRequest::Player(request)) => {
                PlayerConnectionStatus::Active(request)
            }
            ConnectionStatus::Active(_) => PlayerConnectionStatus::InvalidMessage,
            ConnectionStatus::Idle => PlayerConnectionStatus::Idle,
            ConnectionStatus::ReadFailed => PlayerConnectionStatus::ReadFailed,
            ConnectionStatus::InvalidMessage => PlayerConnectionStatus::InvalidMessage,
        }
    }
}
