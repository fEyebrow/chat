use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::{ mpsc };

type Tx = mpsc::UnboundedSender<String>;

pub struct Shared {
  pub peers: HashMap<SocketAddr, Tx>,
}

impl Shared {
  pub fn new() -> Self {
      Shared {
          peers: HashMap::new(),
      }
  }

  pub async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
      for peer in self.peers.iter_mut() {
          if *peer.0 != sender {
              let _ = peer.1.send(message.into());
          }
      }
  }
}
