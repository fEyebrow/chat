use tokio::{net::TcpSocket, sync::{ Mutex }};
use tokio::net::{TcpStream};
use std::io;
use std::sync::Arc;
use crate::shared::{Shared};
use tokio_tungstenite::{ WebSocketStream };
use futures_channel::mpsc::{unbounded, UnboundedReceiver};

type Rx = UnboundedReceiver<String>;

pub struct Peer {
    pub ws: WebSocketStream<TcpStream>,
    pub rx: Rx,
}

impl Peer {
    pub async fn new(
        state: Arc<Mutex<Shared>>,
        ws: WebSocketStream<TcpStream>,
    ) -> io::Result<Peer> {
        let addr = ws.get_ref().peer_addr()?;

        let (tx, rx) = unbounded();
        state.lock().await.peers.insert(addr, tx);

        Ok(Peer{ ws, rx })
    }
}