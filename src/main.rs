#![warn(rust_2018_idioms)]
use std::env;
use tracing::{info};

#[tokio::main]
async fn main() {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:1642".to_string());

    if let Err(e) = chat::run(&addr).await {
        info!("an error occured; error = {:?}", e);
    }
}

