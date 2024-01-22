use anyhow::{anyhow, Context, Result};
use argon2::{Argon2, ParamsBuilder};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub type BlockHash = [u8; 16];

lazy_static! {
    pub static ref GENESIS_BLOCK_HASH: BlockHash = BlockChain::GENESIS_BLOCK
        .hash()
        .context("Failed to calculate hash for the GENESIS_BLOCK")
        .unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockChain {
    pub latest_block_hash: BlockHash,
    pub tree: HashMap<BlockHash, Block>,
}

impl BlockChain {
    const GENESIS_BLOCK: Block = Block { timestamp: 0 };

    pub fn new() -> Result<BlockChain> {
        Ok(BlockChain {
            latest_block_hash: *GENESIS_BLOCK_HASH,
            tree: HashMap::from([(*GENESIS_BLOCK_HASH, Self::GENESIS_BLOCK)]),
        })
    }

    pub fn from_path(path: &Path) -> Result<BlockChain> {
        let data = fs::read(path).context("Failed to read the local blockchain")?;
        let blockchain =
            bincode::deserialize(&data).context("Failed to parse the local blockchain")?;
        Ok(blockchain)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    // pub previous_hash: BlockHash,
    pub timestamp: u64,
    // pub transaction: Transaction,
    // pub proof_of_work: Option<Vec<u8>>,
    // pub miner: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub author: Vec<u8>,
    pub recipient: Vec<u8>,
    pub amount: f64,
    pub signature: Vec<u8>,
}

impl Block {
    pub fn hash(&self) -> Result<BlockHash> {
        let mut hash: BlockHash = [0u8; 16];
        let parameters = ParamsBuilder::new()
            .m_cost(9765) // ~ 10M
            .t_cost(5)
            .p_cost(4)
            .output_len(16)
            .build()
            .unwrap();
        let argon = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            parameters,
        );

        argon
            .hash_password_into(
                &[self.timestamp.to_be_bytes()].concat(),
                b"sprigpow",
                &mut hash,
            )
            .map_err(|err| anyhow!(err))?;

        Ok(hash)
    }
}
