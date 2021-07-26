use tokio::sync::{ mpsc, Mutex };
use tokio_util::codec::{ Framed, LinesCodec };
use tokio::net::{TcpStream};
use std::io;
use std::sync::Arc;
use crate::shared::{Shared};


type Rx = mpsc::UnboundedReceiver<String>;

pub struct Peer {
    pub lines: Framed<TcpStream, LinesCodec>,
    pub rx: Rx,
}


impl Peer {
    pub async fn new(
        state: Arc<Mutex<Shared>>,
        lines: Framed<TcpStream, LinesCodec>
    ) -> io::Result<Peer> {
        let addr = lines.get_ref().peer_addr()?;

        let ( tx, rx ) = mpsc::unbounded_channel();
        state.lock().await.peers.insert(addr, tx);

        Ok(Peer{ lines, rx })
    }
}