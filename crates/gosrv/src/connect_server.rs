use crate::{ConnectServerMessage, ControlMessage, Throttle};
use std::{net::TcpListener, sync::mpsc, time::Duration};

/// Listens for incoming TCP connections
pub struct ConnectServer {
    address: String,
    listener: TcpListener,
    throttle: Throttle,
}

impl ConnectServer {
    pub fn new(address: String) -> eyre::Result<Self> {
        let listener = TcpListener::bind(&address)?;
        listener.set_nonblocking(true)?;
        log::info!("ConnectServer listening on {}", address);

        let throttle = Throttle::new(Duration::from_millis(1));

        Ok(Self {
            address,
            listener,
            throttle,
        })
    }

    pub fn exec_blocking(
        &mut self,
        rx_control: mpsc::Receiver<ControlMessage>,
        tx: mpsc::Sender<ConnectServerMessage>,
    ) {
        loop {
            // Handle control messages
            loop {
                match rx_control.try_recv() {
                    Ok(ControlMessage::Terminate) => {
                        return;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("internal channel must be alive");
                    }
                }
            }

            // Accept incoming connections
            match self.listener.accept() {
                Ok((stream, _addr)) => {
                    if let Err(err) = tx.send(ConnectServerMessage::Incoming(stream)) {
                        log::error!("Failed to send incoming stream: {err}");
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(err) => {
                    log::error!("Connection failed: {err}");
                }
            }

            self.throttle.throttle();
        }
    }
}
