use serde::{Serialize, Deserialize};
use chrono::prelude::*;
use openssl::{sha::sha256, base64};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Record {
    // Number of the record within the block
    pub idx: u64,
    // UTC timestamp
    pub timestamp: u64,
    // Content of the record
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Block {
    pub idx: u64,
    pub previous_block_hash: String,
    // List of n hashes of previous blocks chosen at random.
    // The number of hashes is defined by the network.
    // If the idx of the block is less than the number of hashes defined by the network,
    // the block contains all the hashes of the previous blocks and the rest of the
    // hashes are empty.
    pub validation_hashes: Vec<String>,
    // Proof of Work
    pub pow: String,
    // UTC timestamp
    pub timestamp: u64,
    // List of data records added to the block
    pub records: Vec<Record>,
    // Abstract difficulty value of mining a block. Proof of work is used to find a nonce
    // such that the hash of (data||nonce) is less than 2^hash_output_length/difficulty.
    pub difficulty: Vec<u8>,
}

// Genesis block
impl Block {
    pub fn genesis() -> Block {
        Block {
            idx: 1,
            previous_block_hash: "0".repeat(32),
            validation_hashes: Vec::new(),
            pow: "".to_string(),
            timestamp: 0,
            records: Vec::new(),
            difficulty: vec![0; 32],
        }
    }

    pub fn new(idx: u64,
        previous_hash: String,
        validation_hashes: Vec<String>,
        pow: String,
        records: Vec<Record>,
        difficulty: Vec<u8>,
    ) -> Block
    {
        Block {
            idx,
            previous_block_hash: previous_hash,
            validation_hashes,
            pow,
            timestamp: Utc::now().timestamp() as u64,
            records,
            difficulty,
        }
    }

    // Returns the base64 encoded SHA-256 hash of the block
    pub fn hash(&self) -> String {
        let data = serde_json::json!(self);
        let hash_bytes = sha256(&data.to_string().as_bytes());
        base64::encode_block(hash_bytes.as_ref())
    }
}


