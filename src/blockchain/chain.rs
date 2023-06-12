use crate::blockchain::block::Block;
use openssl::{sha::sha256, base64};
use rand::Rng;
use serde::{Serialize, Deserialize};
use std::fs::File;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Chain {
    pub blocks: Vec<Block>,
    // Abstract difficulty value of mining a block. Proof of work is used to find a nonce
    // such that the hash of (data||nonce) is less than 2^hash_output_length/difficulty.
    pub difficulty: f64,
    // Parameter determining the number of hashes of previous blocks to be included in the
    // block. The number of hashes is defined by the network. If the idx of the block is
    // less than the number of hashes defined by the network, the block contains all the
    // hashes of the previous blocks and the rest of the hashes are empty.
    pub num_side_links: usize,
}

pub enum ChainType {
    Local,
    Remote,
    NoChain,
}

impl Chain {
    pub fn new(difficulty: f64, num_side_links: usize) -> Chain {
        Chain {
            blocks: Vec::new(),
            difficulty,
            num_side_links,
        }
    }

    pub fn load_from_file(file_name: &str) -> Chain {
        let file = std::fs::File::open(file_name).expect("Unable to open the file");
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).expect("Unable to parse the file")
    }

    pub fn save_to_file(&self, file_name: &str) {
        let file = File::create(file_name).expect("Unable to create the file");
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self).expect("Unable to write to the file");
    }

    pub fn init_first_block(&mut self) {
        self.blocks.push(Block::genesis());
    }

    pub fn add_block(&mut self, block: Block) {
        if !self.validate_block(&block) {
            eprintln!("Invalid block: {:?}", block);
            return;
        }
        self.blocks.push(block);
    }

    pub fn choose_random_block_hashes(&self) -> Vec<String> {
        let hashes_to_choose = if self.blocks.len() < self.num_side_links {
            self.blocks.len()
        } else {
            self.num_side_links
        };
        let mut rng = rand::thread_rng();
        let mut hashes = Vec::new();
        let mut i = 0;
        while i < hashes_to_choose {
            let idx = rng.gen_range(0..self.blocks.len());
            let hash = self.blocks[idx].hash();
            hashes.push(hash);
            i += 1;
        }
        hashes
    }

    /*
        Proof ow Work
        A PoWd(data) = b with difficulty d over data is a bit string b s.t.
            token = H(H(data)||b)
            token < (2^l)/d (which is equivalent to:) token/2^l < 1/d
        where H is a cryptographic hash function, e.g. SHA-256, and l is the bit length of token
        (i.e. the output bit length of your hash function). Select an appropriate bit length of b
        (consider the probability that no string of this length produces an output satisfying the
        required property).
    */
    pub fn prove_the_work(&self, data: &str) -> String {
        println!("Proving the work... (mining a block)");
        // Generate a random initial nonce so that the work of every node would not just be
        // a race of who can find the lowest nonce the fastest.
        let mut nonce = rand::thread_rng().gen::<u64>();
        // let difficulty_value: &[u8] = &(2.0f64.powi(256) / self.difficulty as f64).to_be_bytes();
        let difficulty_value: &[u8] = &(self.difficulty).to_be_bytes();
        let mut token: &[u8];
        loop {
            let hash_result = sha256(&[data.as_bytes(), &nonce.to_be_bytes()].concat());
            token = hash_result.as_slice();
            if token.cmp(difficulty_value) == std::cmp::Ordering::Less {
                print!("Found a valid nonce: {}. Result token: {:?} < {:?}",
                    nonce, token, difficulty_value);
                break;
            }
            if nonce % 100000 == 0 {
                println!("Mining... Current nonce: {}.", nonce);
            }
            nonce += 1;
        }

        nonce.to_string()
    }

    pub fn choose_longest_chain(&mut self, remote_chain: &Chain) {
        println!("Choosing the longest chain...");
        let chain_type = self.find_longest_chain(remote_chain);
        match chain_type {
            ChainType::Local => {
                println!("Choosing the local chain.");
            },
            ChainType::Remote => {
                println!("Choosing the remote chain.");
                self.blocks = remote_chain.blocks.clone();
            },
            ChainType::NoChain => {
                panic!("No valid chain to choose from.");
            },
        }
    }

    fn validate_block(&self, block: &Block) -> bool {
        // Check if the block is the genesis block
        if block.idx == 1 {
            if block != &Block::genesis() {
                eprintln!("Verification of the genesis block failed. \
                    Invalid data stored in the genesis block.");
                return false;
            }
            return true;
        }
        let last_confirmed_block = self.blocks.last()
            .expect("No blocks in the chain but block.idx != 1");

        // Check the correctness of ID of the block
        if block.idx != last_confirmed_block.idx + 1 {
            eprintln!("Verification of block with ID {}. \
                Invalid ID of the block; should be: {}",
                block.idx, last_confirmed_block.idx + 1);
            return false;
        }

        // Check if the block is the next block in the chain
        let last_block_hash = last_confirmed_block.hash();
        if block.previous_hash != last_block_hash {
            eprintln!("Verification of block with ID {}. \
                Invalid hash of the previous block: stored: {:?}, actual hash: {:?}",
                block.idx, block.previous_hash, last_block_hash);
            return false;
        }

        // Check the proof of work
        let difficulty_value: &[u8] = &(2.0f64.powi(256) / self.difficulty as f64).to_be_bytes();
        let hash_result = sha256(&[block.hash().as_bytes(), block.pow.as_bytes()].concat());
        let token = hash_result.as_slice();
        if token.cmp(difficulty_value) != std::cmp::Ordering::Less {
            eprintln!("Verification of block with ID {}. \
                Invalid proof of work: {:?} >= {:?}",
                block.idx, token, difficulty_value);
            return false;
        }

        // TODO: Check the number of hashes of previous blocks?

        true
    }

    fn validate_chain(&self) -> bool {
        // Check if the chain is empty
        if self.blocks.is_empty() {
            eprintln!("Verification of the chain failed. \
                The chain is empty.");
            return false;
        }

        // Check if the genesis block is correct
        if self.blocks[0] != Block::genesis() {
            eprintln!("Verification of the chain failed. \
                The genesis block is incorrect.");
            return false;
        }

        // Check if the chain is continuous
        for i in 1..self.blocks.len() {
            if !self.validate_block(&self.blocks[i]) {
                eprintln!("Verification of the chain failed. \
                    Block with ID {} is invalid.", i + 1);
                return false;
            }
        }

        true
    }

    // Mechanism for choosing the longest chain
    fn find_longest_chain(&mut self, remote_chain: &Chain) -> ChainType {
        let local_chain_validation = self.validate_chain();
        let remote_chain_validation = remote_chain.validate_chain();
        if local_chain_validation && remote_chain_validation {
            if self.blocks.len() > remote_chain.blocks.len() {
                ChainType::Local
            } else if self.blocks.len() < remote_chain.blocks.len() {
                ChainType::Remote
            } else {
                // Return the chain with the lowest hash value of the last block if chains have
                // equal length
                let local_last_block_hash = self.blocks.last().unwrap().hash();
                let local_last_block_hash = base64::decode_block(&local_last_block_hash).unwrap();
                let remote_last_block_hash = remote_chain.blocks.last().unwrap().hash();
                let remote_last_block_hash = base64::decode_block(&remote_last_block_hash).unwrap();
                
                if local_last_block_hash <= remote_last_block_hash {
                    ChainType::Local
                } else {
                    ChainType::Remote
                }
            }
        } else if local_chain_validation {
            eprintln!("Verification of the remote chain failed. \
                The remote chain is invalid.");
            ChainType::Local
        } else if remote_chain_validation {
            eprintln!("Verification of the current chain failed. \
                The current chain is invalid.");
            ChainType::Remote
        } else {
            eprintln!("Verification of the current adn remote chain failed.");
            ChainType::NoChain
        }
    }
}
