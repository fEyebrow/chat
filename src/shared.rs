use dashmap::DashMap;
use std::net::SocketAddr;
use futures_channel::mpsc::{UnboundedSender};

type Tx = UnboundedSender<String>;

pub struct Shared {
  pub peers: DashMap<SocketAddr, Tx>,
}

impl Shared {
  pub fn new() -> Self {
      Shared {
          peers: DashMap::new(),
      }
  }

  pub async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
    for peer in self.peers.iter() {
      if *peer.key() != sender {
          let _ = peer.value().unbounded_send(message.into());
      }
    }
  }
}
