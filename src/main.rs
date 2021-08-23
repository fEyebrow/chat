use tracing::{info};

#[tokio::main]
async fn main() {

    if let Err(e) = chat::run().await {
        info!("an error occured; error = {:?}", e);
    }
}

