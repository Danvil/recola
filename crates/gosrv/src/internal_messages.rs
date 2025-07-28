use crate::PlayerConnection;
use std::net::TcpStream;

pub enum ControlMessage {
    Terminate,
}

pub enum ConnectServerMessage {
    Incoming(TcpStream),
}

pub enum LoginServerMessage {
    Login(PlayerConnection),
}

pub enum GameServerMessage {
    Kick(PlayerConnection),
    Terminated,
}
