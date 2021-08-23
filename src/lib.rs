use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::net::ToSocketAddrs;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;


use futures_util::{SinkExt, StreamExt};
use tokio_rustls::TlsAcceptor;
use tungstenite::protocol::Message;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{ Mutex };
use tracing::{info, debug};
use tracing_subscriber;
use tokio_rustls::rustls::internal::pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{ Certificate, NoClientAuth, PrivateKey, ServerConfig };
use argh::FromArgs;

pub mod shared;
pub use shared::{Shared};
pub mod peer;
pub use peer::{ Peer };
pub mod room;
pub use room::{ Room };

/// tokio tls server
#[derive(FromArgs)]
pub struct Options {
    ///bind addr
    #[argh(positional)]
    pub addr: String,

    /// cert file
    #[argh(option, short = 'c', default="PathBuf::from(r\"localhost.crt\")")]
    pub cert: PathBuf,

    /// key file
    #[argh(option, short =  'k', default="PathBuf::from(r\"localhost.key\")")]
    pub key: PathBuf,
}

fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|e| {
            tracing::error!("invalid cert {:?}", e);
            io::Error::new(io::ErrorKind::InvalidInput, "invalid cert")
        })
        // .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
}

fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    pkcs8_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|e| {
            tracing::error!("invalid key {:?}", e);
            io::Error::new(io::ErrorKind::InvalidInput, "invalid key")
        })
}

pub async fn run() ->  Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let options: Options = argh::from_env();

    let addr = options
        .addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;


    let certs = load_certs(&options.cert)?;
    let mut keys = load_keys(&options.key)?;
    let mut config = ServerConfig::new(NoClientAuth::new());
    config.set_single_cert(certs, keys.remove(0))
          .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let rooms = init_rooms();

    let listener = TcpListener::bind(&addr).await?;

    info!("server running on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await?;

        let rooms = Arc::clone(&rooms);
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            debug!("accepted connection");
            if let Err(e) = process(rooms, stream, addr, acceptor).await {
                info!("an error occured; error = {:?}", e);
            }
        });
    }
}

fn init_rooms() -> Arc<Vec<Room>> {
    let state1 = Arc::new(Mutex::new(Shared::new()));
    let state2 = Arc::new(Mutex::new(Shared::new()));
    Arc::new(vec![Room::new(String::from("pub"), state1), Room::new(String::from("dota2"), state2)])
}


async fn process(rooms: Arc<Vec<Room>>, stream: TcpStream, addr: SocketAddr, acceptor: TlsAcceptor)
-> Result<(), Box<dyn Error>> {
    println!("Incoming TCP connection from: {}", addr);
    let mut stream = acceptor.accept(stream).await?;

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    ws_stream.send(Message::Text("please enter your username:".to_owned())).await?;

    let username = match ws_stream.next().await {
        Some(msg) => msg.unwrap(),
        _ => {
            tracing::error!("Failed to get username from {}. Client disconnected.", addr);
            return Ok(());
        },
    };
    println!("username: {}", username);

    ws_stream.send(Message::Text("please select a chat room\n- pub\n- dota2".to_owned())).await?;
    let room_key = match ws_stream.next().await {
        Some(msg) => msg.unwrap(),
        _ => {
            tracing::error!("Failed to get the name of room");
            return Ok(());
        },
    };

    let mut state: Option<Arc<Mutex<Shared>>> = None;
    for room in rooms.iter() {
        if room.key == room_key.to_string() {
            state = Some(Arc::clone(&room.state));
        }
    }
    if let None = state {
        tracing::error!("Failed to find the room selected");
        return Ok(());
    }
    ws_stream.send(Message::Text(format!("joined to {} room", room_key).to_owned())).await?;

    let state = state.unwrap();

    let mut peer = Peer::new(state.clone(), ws_stream).await?;

    {
        let mut state = state.lock().await;
        let msg = format!("{} has joined the chat", username);
        info!("{}", msg);
        state.broadcast(addr, &msg).await;
    }

    let (mut outgoing,mut incoming) = peer.ws.split();
    loop {
        tokio::select! {
            msg = peer.rx.next() => {
                outgoing.send(Message::Text(msg.unwrap())).await?;
            }

            msg = incoming.next() => match msg {
                Some(msg) => {
                    let mut state = state.lock().await;
                    let msg = format!("{}: {}", username, msg.unwrap().to_string());

                    state.broadcast(addr, &msg).await
                },
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