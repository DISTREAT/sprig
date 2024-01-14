use crate::blockchain::{BlockChain, BlockHash};
use anyhow::{Context, Result};
use crossbeam::channel::{unbounded, Sender};
use crossbeam::select;
use kadcast::{config::Config, MessageInfo, NetworkListen, Peer};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

const BLOCKCHAIN_PATH_RAW: &str = "./sprigchain.data";

#[derive(Debug, Serialize, Deserialize)]
enum SChainMessage {
    Request(SChainRequest),
    Response(SocketAddr, SChainResponse),
}

#[derive(Debug, Serialize, Deserialize)]
enum SChainRequest {
    SearchingBlock { parent_hash: BlockHash },
}

#[derive(Debug, Serialize, Deserialize)]
enum SChainResponse {
    InvalidRequest,
}

struct SChainListener {
    blockchain: Arc<Mutex<BlockChain>>,
    peer_event_send: Sender<(SocketAddr, Vec<u8>)>,
}

impl NetworkListen for SChainListener {
    fn on_message(&self, message: Vec<u8>, message_info: MessageInfo) {
        let result: serde_json::Result<SChainMessage> = serde_json::from_slice(&message);

        match result {
            Ok(message) => match message {
                SChainMessage::Request(request) => self.on_request(message_info.src(), request),
                SChainMessage::Response(src, response) => self.on_response(src, response),
            },
            Err(_) => self
                .peer_event_send
                .send((
                    message_info.src(),
                    serde_json::to_vec(&SChainMessage::Response(
                        message_info.src(),
                        SChainResponse::InvalidRequest,
                    ))
                    .expect("Failed to serialize response to json"),
                ))
                .expect("Failed to queue peer_send event"),
        }
    }
}

impl SChainListener {
    fn on_request(&self, src: SocketAddr, request: SChainRequest) {
        match request {
            SChainRequest::SearchingBlock { parent_hash } => {
                info!("{src:?} needs block after {parent_hash:?}");
            }
        }
    }

    fn on_response(&self, src: SocketAddr, response: SChainResponse) {
        match response {
            SChainResponse::InvalidRequest => warn!("Peer '{src:?}' reported an InvalidRequest"),
        }
    }
}

pub async fn server(
    public_address: String,
    listen_address: Option<String>,
    bootstrapping_nodes: Vec<String>,
) -> Result<()> {
    let blockchain_path = PathBuf::from(BLOCKCHAIN_PATH_RAW);
    let blockchain = Arc::new(Mutex::new(if blockchain_path.exists() {
        BlockChain::from_path(&blockchain_path)?
    } else {
        BlockChain::new()?
    }));

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish(),
    )?;

    let config = Config {
        public_address,
        listen_address,
        bootstrapping_nodes,
        ..Default::default()
    };

    let (peer_event_send_sender, peer_event_send_receiver) = unbounded();
    let peer = Peer::new(
        config,
        SChainListener {
            blockchain: blockchain.clone(),
            peer_event_send: peer_event_send_sender,
        },
    )
    .context("Failed to initialize a kadcast server")?;

    let mut timer = Instant::now();
    let mut initial_syncronization = true;
    let blockchain = blockchain.clone();
    loop {
        select! {
            recv(peer_event_send_receiver) -> result => if let Ok((src, message)) = result { peer.send(&message, src).await},
            default => {
                if timer.elapsed().as_secs() >= if initial_syncronization { 1} else {10} {
                    timer = Instant::now();

                    let blockchain = blockchain.lock().await;
                    let target = peer.alive_nodes(1).await;

                    if !target.is_empty() {
                        peer.send(
                            &serde_json::to_vec(&SChainMessage::Request(SChainRequest::SearchingBlock{parent_hash: blockchain.latest_block_hash}))?,
                            target[0]
                        ).await;
                    }
                }
            }
        }
    }
}
