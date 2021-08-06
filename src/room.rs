use crate::shared::{Shared};
use std::sync::Arc;
use tokio::sync::{ Mutex };

pub struct Room {
  pub key: String,
  pub state: Arc<Mutex<Shared>>,
}

impl Room {
  pub fn new(key: String, state: Arc<Mutex<Shared>>) -> Self {
    Self {
      key,
      state
    }
  }
}