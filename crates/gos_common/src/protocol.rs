use gosim::Order;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientRequest {
    Login(requests::Login),
    Player(PlayerRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PlayerRequest {
    CreateOrder(Order),
    // Craft {
    //     id: Symbol,
    //     quantity: u32,
    // },
    // Consume {
    //     id: Symbol,
    //     quantity: u32,
    // },
    // GetState,
}

pub mod requests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Login {
        pub steam_user: u64,
        pub steam_auth: Vec<u8>,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerResponse {
    Ok,
    Error(String),
    // State(PlayerState),
}
