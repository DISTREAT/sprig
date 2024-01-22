use crate::node::Storage;
use anyhow::{Context, Result};
use crossbeam::channel::{unbounded, Sender};
use crossbeam::select;
use kadcast::{config::Config, MessageInfo, NetworkListen, Peer};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::{self, JoinHandle};
use tracing::{error, info, warn};

type PeerMessage = (PeerMessageSender, PeerMessageContent);
type PeerMessageSender = SocketAddr;
type PeerMessageContent = Vec<u8>;

// const BLOCKCHAIN_PATH_RAW: &str = "./sprigchain.data";

struct Listener {
    storage: Arc<Mutex<Storage>>,
    peer_message_sender: Sender<PeerMessage>,
}

impl NetworkListen for Listener {
    fn on_message(&self, message: Vec<u8>, message_info: MessageInfo) {
        let result: serde_json::Result<protocol::Message> = serde_json::from_slice(&message);

        match result {
            Ok(message) => match message {
                protocol::Message::Request(request) => self.on_request(message_info.src(), request),
                protocol::Message::Response(src, response) => self.on_response(src, response),
            },
            Err(_) => self
                .peer_message_sender
                .send((
                    message_info.src(),
                    serde_json::to_vec(&protocol::Message::Response(
                        message_info.src(),
                        protocol::Response::InvalidRequest,
                    ))
                    .expect("Failed to serialize response to json"),
                ))
                .expect("Failed to queue peer message"),
        }
    }
}

impl Listener {
    fn on_request(&self, src: SocketAddr, request: protocol::Request) {
        match request {
            protocol::Request::SearchingBlock { parent_hash } => {
                info!("{src:?} needs block after {parent_hash:?}");
            }
            _ => unimplemented!(),
        }
    }

    fn on_response(&self, src: SocketAddr, response: protocol::Response) {
        match response {
            protocol::Response::InvalidRequest => {
                warn!("Peer '{src:?}' reported an InvalidRequest")
            }
        }
    }
}

mod protocol {
    use crate::blockchain::{BlockHash, Transaction};
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Message {
        Request(Request),
        Response(SocketAddr, Response),
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Request {
        SearchingBlock { parent_hash: BlockHash },
        BroadcastPendingTransaction(Transaction),
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Response {
        InvalidRequest,
    }
}

pub struct P2PNetwork {
    storage: Arc<Mutex<Storage>>,
    peer: Arc<Peer>,
    thread: JoinHandle<()>,
}

impl P2PNetwork {
    pub fn new(
        storage: Arc<Mutex<Storage>>,
        public_address: String,
        listen_address: Option<String>,
        bootstrapping_nodes: Vec<String>,
    ) -> Result<P2PNetwork> {
        let config = Config {
            public_address,
            listen_address,
            bootstrapping_nodes,
            ..Default::default()
        };

        let (peer_message_sender, peer_message_receiver) = unbounded();
        let peer = Arc::new(
            Peer::new(
                config,
                Listener {
                    storage: storage.clone(),
                    peer_message_sender,
                },
            )
            .context("Failed to initialize a kadcast instance")?,
        );

        let thread_peer = peer.clone();
        let thread = task::spawn(async move {
            select! {
                recv(peer_message_receiver) -> result => if let Ok((src, message)) = result { thread_peer.send(&message, src).await},
            }
        });

        Ok(P2PNetwork {
            storage,
            peer: peer.clone(),
            thread,
        })
    }

    pub async fn sync_blockchain(&self) -> Result<()> {
        let storage = self.storage.lock().await;
        let target = self.peer.alive_nodes(1).await;

        if !target.is_empty() {
            self.peer
                .send(
                    &serde_json::to_vec(&protocol::Message::Request(
                        protocol::Request::SearchingBlock {
                            parent_hash: storage.blockchain.latest_block_hash,
                        },
                    ))?,
                    target[0],
                )
                .await;
        }

        Ok(())
    }
}
