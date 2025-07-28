use gos_common::{requests, ClientRequest, PlayerRequest};
use std::{io::Write, net::TcpStream};
use steamworks as sw;

pub struct Client {
    steam_client: sw::Client,
    stream: TcpStream,
}

impl Client {
    pub fn new() -> eyre::Result<Self> {
        let steam_client = sw::Client::init()?;

        let stream = TcpStream::connect("127.0.0.1:9000")?;

        Ok(Self {
            steam_client,
            stream,
        })
    }

    pub fn authenticate(&mut self) -> eyre::Result<()> {
        let steam_user = self.steam_client.user();

        let buffer = bincode::serde::encode_to_vec(
            ClientRequest::Login(requests::Login {
                steam_user: steam_user.steam_id().raw(),
                steam_auth: steam_user
                    .authentication_session_ticket_with_steam_id(steam_user.steam_id())
                    .1,
            }),
            bincode::config::standard(),
        )?;

        self.stream.write_all(&buffer)?;

        Ok(())
    }

    pub fn send_player_request(&mut self, request: PlayerRequest) -> eyre::Result<()> {
        let buffer = bincode::serde::encode_to_vec(
            ClientRequest::Player(request),
            bincode::config::standard(),
        )?;

        self.stream.write_all(&buffer)?;

        Ok(())
    }
}
