use serde::{Serialize, Deserialize};
use chrono::prelude::*;
use openssl::{sha::sha256, base64};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Record {
    // Number of the record within the block
    pub idx: (u64, u64),
    // UTC timestamp
    pub timestamp: u64,
    // Content of the record
    pub data: String,
    pub author_peer_id: String,
}

impl Record {
    pub fn new(data: String, author_peer_id: String) -> Record {
        Record {
            idx: (0, 0),
            timestamp: Utc::now().timestamp() as u64,
            data,
            author_peer_id,
        }
    }
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
    pub validation_sidelinks: Vec<String>,
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
            validation_sidelinks: Vec::new(),
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
            validation_sidelinks: validation_hashes,
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

    #[allow(dead_code)]
    pub fn add_record_by_data(&mut self, data: String, author_peer_id: String) {
        let last_record = self.records.last();
        let idx = if let Some(last_record) = last_record {
            (self.idx, last_record.idx.1 + 1)
        } else {
            (self.idx, 1)
        };

        let new_rec = Record {
            idx,
            timestamp: Utc::now().timestamp() as u64,
            data,
            author_peer_id,
        };

        self.records.push(new_rec);
    }

    pub fn add_record(&mut self, mut record: Record) {
        let last_record = self.records.last();
        let idx = if let Some(last_record) = last_record {
            (self.idx, last_record.idx.1 + 1)
        } else {
            (self.idx, 1)
        };
        record.idx = idx;
        self.records.push(record);
    }

    #[allow(dead_code)]
    // This function is generally wrong but stays in code as a concept to get fixed some day
    pub fn derive_sidelink_indices_bad(&self, num_sidelinks: usize) -> Vec<usize> {
        let mut indices = Vec::new();
        // Derive num_sidlink indices from the previous block hash, this is deterministic
        // and will always return the same set of unique indices for the same block hash.
        let hash = self.previous_block_hash.clone();
        if num_sidelinks < (self.idx - 1) as usize {
            for i in 0..num_sidelinks {
                // Concatenate the previous block hash with the index of the sidelink
                let hash_bytes = sha256(&format!("{}{}", hash, i).as_bytes());
                // If there is a collision (i.e. we
                // derive an index which is already present in the block) and, for example,
                // sidelink a is equal to sidelink b, where a was calculated earlier than b,
                // then for b the sidelink will be a-1
                let mut idx = u64::from_be_bytes(hash_bytes[24..].try_into().unwrap()) % (self.idx - i as u64) as u64;
                println!("derived idx: {}", idx);
                let idx_of_same_value = indices.iter().position(|&x| x == idx as usize);

                if let Some(idx_of_same_value) = idx_of_same_value {
                    println!("Already derived {idx} for sidelink number {idx_of_same_value}. \
                        Setting the new sidelink to {}", num_sidelinks - idx_of_same_value - 1);
                    idx = (num_sidelinks - idx_of_same_value - 1) as u64;
                }
                indices.push(idx as usize);
            }
        } else {
            // If the number of sidelinks is greater than the block index, then the block
            // contains all the previous block hashes.
            indices = (0..(self.idx - 1) as usize).collect();
        }

        indices
    }

    pub fn derive_sidelink_indices(&self, num_sidelinks: usize) -> Vec<usize> {
        let mut candidates = (0..(self.idx - 1) as usize).collect::<Vec<usize>>();

        if num_sidelinks < (self.idx - 1) as usize {
            let hash = self.previous_block_hash.clone();

            // Perform deterministic swaps based on the previous block hash
            // The number of swaps is arbitrary
            let number_of_swaps = num_sidelinks * 2;

            for i in 0..number_of_swaps {
                let hash_bytes1 = sha256(&format!("{}{}", hash, i).as_bytes());
                let hash_bytes2 = sha256(&format!("{}{}{}", hash, i, i).as_bytes());

                let idx1 = u64::from_be_bytes(hash_bytes1[24..].try_into().unwrap()) % (num_sidelinks as u64) as u64;
                let idx2 = u64::from_be_bytes(hash_bytes2[24..].try_into().unwrap()) % (num_sidelinks as u64) as u64;

                let tmp = candidates[idx1 as usize];
                candidates[idx1 as usize] = candidates[idx2 as usize];
                candidates[idx2 as usize] = tmp;
            }

            candidates[candidates.len() - num_sidelinks - 1..].to_vec()
        } else {
            candidates
        }
    }

}


