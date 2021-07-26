use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{ Mutex };
use tokio_stream::StreamExt;
use tokio_util::codec::{ Framed, LinesCodec };

use tracing::{info, debug};
use tracing_subscriber;


use futures::SinkExt;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

pub mod shared;
pub use shared::{Shared};
pub mod peer;
pub use peer::{ Peer };


pub async fn run(addr: &str) ->  Result<(), Box<dyn Error>> {
  tracing_subscriber::fmt::init();

    let state = Arc::new(Mutex::new(Shared::new()));

    let listener = TcpListener::bind(&addr).await?;

    info!("server running on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;

        let state = Arc::clone(&state);

        tokio::spawn(async move {
            debug!("accepted connection");
            if let Err(e) = process(state, stream, addr).await {
                info!("an error occured; error = {:?}", e);
            }
        });
    }
}


async fn process(state: Arc<Mutex<Shared>>, stream: TcpStream, addr: SocketAddr)
-> Result<(), Box<dyn Error>> {
    let mut lines = Framed::new(stream, LinesCodec::new());
    lines.send("please enter your username:").await?;

    let username = match lines.next().await {
        Some(Ok(line)) => line,
        _ => {
            tracing::error!("Failed to get username from {}. Client disconnected.", addr);
            return Ok(());
        },
    };

    let mut peer = Peer::new(state.clone(), lines).await?;

    {
        let mut state = state.lock().await;
        let msg = format!("{} has joined the chat", username);
        info!("{}", msg);
        state.broadcast(addr, &msg).await;
    }

    loop {
        tokio::select! {
            Some(msg) = peer.rx.recv() => {
                peer.lines.send(&msg).await?;
            }
            result = peer.lines.next() => match result {
                Some(Ok(msg)) => {
                    let mut state = state.lock().await;
                    let msg = format!("{}: {}", username, msg);

                    state.broadcast(addr, &msg).await
                },
                Some(Err(e)) => {
                    tracing::error!(
                        "an error occured while processing message for {}; error = {:?}",
                        username,
                        e
                    )
                }
                None => break,
            }
        }
    }

    {
        let mut state = state.lock().await;
        state.peers.remove(&addr);

        let msg = format!("{} has left the chat", username);
        info!("{}", msg);
        state.broadcast(addr, &msg).await;
    }

    Ok(())
}