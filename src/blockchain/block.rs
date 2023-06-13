use serde::{Serialize, Deserialize};
use chrono::prelude::*;
use openssl::{sha::sha256, base64};
use std::fs::File;
use std::io::{self, BufRead};

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
    pub previous_hash: String,
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
}

// Genesis block
impl Block {
    pub fn genesis() -> Block {
        Block {
            idx: 1,
            previous_hash: "0".repeat(256),
            validation_hashes: Vec::new(),
            pow: "".to_string(),
            timestamp: Utc::now().timestamp() as u64,
            records: Vec::new(),
        }
    }

    pub fn new(idx: u64,
        previous_hash: String,
        validation_hashes: Vec<String>,
        pow: String,
        records: Vec<Record>) -> Block
    {
        Block {
            idx,
            previous_hash,
            validation_hashes,
            pow,
            timestamp: Utc::now().timestamp() as u64,
            records,
        }
    }

    // Returns the base64 encoded SHA-256 hash of the block
    pub fn hash(&self) -> String {
        let data = serde_json::json!(self);
        let hash_bytes = sha256(&data.to_string().as_bytes());
        base64::encode_block(hash_bytes.as_ref())
    }

    pub fn load_block_from_file(block_idx: usize, file_name: &str) -> Result<Block, Box<dyn std::error::Error>> {
        // TODO: we assume that the file is not corrupted and that, for simplicity, every
        // block is on separate line. So to get ith block we simply read the ith line.
        let file = File::open(file_name)?;
        let reader = io::BufReader::new(file);

        // Read the file until reaching the desired element index
        for (i, line) in reader.lines().enumerate() {
            if i == block_idx - 1 {
                return Ok(serde_json::from_str(&line?)?);
            }
        }

        Err("Error while reading the file".into())
    }
}


