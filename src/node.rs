use crate::api::ApiServer;
use crate::blockchain::{BlockChain, Transaction};
use crate::network::P2PNetwork;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::ToSocketAddrs;
use tokio::sync::Mutex;

pub struct NodeServerConfig<A: ToSocketAddrs> {
    pub public_address: String,
    pub listen_address: Option<String>,
    pub bootstrapping_nodes: Vec<String>,
    pub api_address: A,
}

pub struct Storage {
    pub minable_transactions: Vec<Transaction>,
    pub blockchain: BlockChain,
}

impl Default for Storage {
    fn default() -> Self {
        Storage {
            minable_transactions: Vec::new(),
            blockchain: BlockChain::new().unwrap(),
        }
    }
}

pub async fn serve<A: ToSocketAddrs>(config: NodeServerConfig<A>) -> Result<()> {
    let storage = Arc::new(Mutex::new(Storage::default()));
    let network = P2PNetwork::new(
        storage.clone(),
        config.public_address,
        config.listen_address,
        config.bootstrapping_nodes,
    )?;
    let api = ApiServer::new(storage.clone(), config.api_address).await?;

    let mut timer = Instant::now();
    let mut initial_syncronization = false;

    loop {
        if timer.elapsed().as_secs() >= if initial_syncronization { 1 } else { 10 } {
            timer = Instant::now();

            network.sync_blockchain().await?;
        }
    }
}
