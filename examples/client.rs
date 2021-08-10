use std::env;
use futures_util::{future, pin_mut, StreamExt, SinkExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{ connect_async, tungstenite::protocol::Message };

#[tokio::main]
async fn main() {
  let connect_addr =
    env::args().nth(1).unwrap_or_else(|| "ws://127.0.0.1:1642/".to_string());

  let url = url::Url::parse(&connect_addr).unwrap();

  let ( mut stdin_tx, mut stdin_rx ) = futures_channel::mpsc::unbounded();
  tokio::spawn(read_stdin(stdin_tx));

  let ( ws_stream, _ ) = connect_async(url).await.expect("Failed to connect");
  let ( mut write, mut read ) = ws_stream.split();

  loop {
    tokio::select! {
      msg = stdin_rx.next() => {
        let msg_ = msg.unwrap();
        write.send(msg_).await.unwrap();
      }

      msg = read.next() => match msg {
        Some(msg) => {
          let text = msg.unwrap().into_text().unwrap();
          println!("{}", text);
          // tokio::io::stdout().write_all(text.as_bytes()).await.unwrap();
        },
        None => break,
      }
    }
  }
}

async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
  let mut stdin = tokio::io::stdin();
  loop {
    let mut buf = vec![0; 1024];
    let n = match stdin.read(&mut buf).await {
      Err(_) | Ok(0) => break,
      Ok(n) => n,
    };
    buf.truncate(n);
    let msg = String::from(std::str::from_utf8(&buf).unwrap().replace("\n", ""));
    tx.unbounded_send(Message::Text(msg));
  }
}