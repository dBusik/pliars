use crate::blockchain::block::Block;
use openssl::{sha::sha256, base64};
use rand::Rng;
use serde::{Serialize, Deserialize/*, Serializer*/};
// use serde::ser::SerializeSeq;
use std::fs::{File, OpenOptions};
use std::io::{self, Write, BufRead};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Chain {
    pub blocks: Vec<Block>,
    // Abstract difficulty value of mining a block. Proof of work is used to find a nonce
    // such that the hash of (data||nonce) is less than 2^hash_output_length/difficulty.
    pub difficulty: Vec<u8>,
    // Parameter determining the number of hashes of previous blocks to be included in the
    // block. The number of hashes is defined by the network. If the idx of the block is
    // less than the number of hashes defined by the network, the block contains all the
    // hashes of the previous blocks and the rest of the hashes are empty.
    pub num_side_links: usize,
}

#[derive(Debug, PartialEq)]
pub enum ChainType {
    Local,
    Remote,
    Both,
    NoChain,
}

#[derive(Debug)]
pub struct ChainChoice {
    pub chosen_chain_type: ChainType,
    pub chosen_chain: Option<Chain>,
}

// Mechanism for choosing the longest chain
pub fn find_longest_chain(local_chain: &Chain, remote_chain: &Chain) -> ChainChoice {
    let local_chain_validation = local_chain.validate_chain();
    let remote_chain_validation = remote_chain.validate_chain();
    let winner_chain_type = if local_chain_validation && remote_chain_validation {
        if local_chain.blocks.len() > remote_chain.blocks.len() {
            ChainType::Local
        } else if local_chain.blocks.len() < remote_chain.blocks.len() {
            ChainType::Remote
        } else {
            // Return the chain with the lowest hash value of the last block if chains have
            // equal length
            let local_last_block_hash = local_chain.blocks.last().unwrap().hash();
            let local_last_block_hash = base64::decode_block(&local_last_block_hash).unwrap();
            let remote_last_block_hash = remote_chain.blocks.last().unwrap().hash();
            let remote_last_block_hash = base64::decode_block(&remote_last_block_hash).unwrap();
            
            if local_last_block_hash < remote_last_block_hash {
                ChainType::Local
            } else if local_last_block_hash == remote_last_block_hash {
                ChainType::Both
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
    };

    println!("Choosing the longest chain...");
    let winner_chain = match winner_chain_type {
        ChainType::Local => {
            println!("Choosing the local chain.");
            Some(local_chain)
        },
        ChainType::Remote => {
            println!("Choosing the remote chain.");
            Some(remote_chain)
        },
        ChainType::Both => {
            println!("Chains are equal.");
            None
        },
        ChainType::NoChain => {
            println!("No valid chain to choose from.");
            None
        },
    };

    return ChainChoice {
        chosen_chain_type: winner_chain_type,
        chosen_chain: winner_chain.cloned()
    };
}

impl Chain {
    pub fn new(difficulty: Vec<u8>, num_side_links: usize) -> Chain {
        Chain {
            blocks: Vec::new(),
            difficulty,
            num_side_links,
        }
    }

    pub fn load_from_file(file_name: &str) -> Result<Chain, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(file_name)?;
        let reader = std::io::BufReader::new(file);
        let mut blocks = Vec::new();
        for line in reader.lines() {
            let block = serde_json::from_str(&line?)?;
            blocks.push(block);
        }
        Ok(Chain {
            blocks,
            // TODO: determine how to store difficulty and num_side_links in the file
            difficulty: Vec::new(),
            num_side_links: 0,
        })
    }

    pub fn save_blockchain_to_file(&self, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(file_name)?;
        // Each block should be serialized on a separate line
        let mut blockchian_string = Vec::new();
        for block in &self.blocks {
            let block_string = serde_json::to_string(block)?;
            // Push a string with a newline character
            blockchian_string.push(format!("{}\n", block_string));
        }
        file.write_all(blockchian_string.join("").as_bytes())?;

        Ok(())
        
        // Alternative way of serializing the blockchain - everything in one line
        // let mut serializer = serde_json::Serializer::new(&file);
        // let mut seq = serializer
        //     .serialize_seq(Some(self.blocks.len())).expect("can serialize sequence");
        // for block in &self.blocks {
        //     seq.serialize_element(block).expect("can serialize element");
        // }
        // seq.end().expect("can end serialization");
        // file.flush().expect("can flush writer");
    }

    pub fn get_blockchain_length(file_name: &str) -> usize {
        // TODO: we assume that the file is not corrupted and that, for simplicity, every
        // block is on separate line. So to get ith block we simply read the ith line.
        let file = File::open(file_name).expect("Unable to open the file");
        let length_reader = io::BufReader::new(file);
        length_reader.lines().count()
    }

    pub fn get_last_block(file_name: &str) -> Option<Block> {
        let blockchain_length = Chain::get_blockchain_length(file_name);
        let mut last_block = None;

        if blockchain_length > 0 {
            last_block = Some(Block::load_block_from_file(blockchain_length - 1, file_name)
                .expect("can load block"));
        }

        last_block
    }

    pub fn append_block_to_file(&self,
        file_name: &str,
        block: &Block
    ) -> Result<(), Box<dyn std::error::Error>>
    {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(file_name)
            .expect("Unable to open the file");

        let block_string = serde_json::to_string(block)?;
        file.write_all(format!("{}\n", block_string).as_bytes())?;

        Ok(())
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
        let hash_result = sha256(&[block.hash().as_bytes(), block.pow.as_bytes()].concat());
        let token = hash_result.as_slice();
        if token.cmp(&self.difficulty) != std::cmp::Ordering::Less {
            eprintln!("Verification of block with ID {}. \
                Invalid proof of work: {:?} >= {:?}",
                block.idx, token, self.difficulty);
            return false;
        }

        // TODO: Check the number of hashes of previous blocks?

        true
    }

    pub fn validate_chain(&self) -> bool {
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
}
