use crate::blockchain::block::Block;
use crate::blockchain::pow;
use openssl::base64;
use rand::Rng;
use serde::{Serialize, Deserialize};
use std::fs::{File, OpenOptions};
use std::io::{self, Write, BufRead};
use log::{info, warn, error};

pub static mut DIFFICULTY_VALUE: Vec<u8> = Vec::new();
pub static mut NUM_SIDELINKS: usize = 5;
pub const DEFAULT_DIFFICULTY_IN_SECONDS: f64 = 30.0;
pub const DEFAULT_NUM_OF_SIDELINKS: usize = 5;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Chain {
    pub blocks: Vec<Block>,
    // Parameter determining the number of hashes of previous blocks to be included in the
    // block. The number of hashes is defined by the network. If the idx of the block is
    // less than the number of hashes defined by the network, the block contains all the
    // hashes of the previous blocks and the rest of the hashes are empty.
    pub num_sidelinks: usize,
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

#[derive(Debug, PartialEq)]
pub enum ChainValidationResult {
    FileError,
    ChainError,
    ChainOk,
}

#[derive(Debug)]
enum BlockValidationSource {
    File,
    Chain,
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
        println!("Verification of the remote chain failed. \
            The remote chain is invalid.");
        ChainType::Local
    } else if remote_chain_validation {
        println!("Verification of the current chain failed. \
            The current chain is invalid.");
        ChainType::Remote
    } else {
        println!("Verification of the current adn remote chain failed.");
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
    pub fn new(num_side_links: usize) -> Chain {
        Chain {
            blocks: Vec::new(),
            num_sidelinks: num_side_links,
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
            num_sidelinks: unsafe { NUM_SIDELINKS },
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

    pub fn get_blockchain_length(file_name: &str) -> Result<usize, Box<dyn std::error::Error>> {
        // TODO: we assume that the file is not corrupted and that, for simplicity, every
        // block is on separate line. So to get ith block we simply read the ith line.
        let file_res = File::open(file_name);
        if let Ok(file) = file_res {
            let length_reader = io::BufReader::new(file);
            Ok(length_reader.lines().count())
        } else {
            warn!("Error while opening the file: {}", file_res.err().unwrap());
            Err("Error while opening the file".into())
        }
    }

    pub fn append_block_to_file(block: &Block,
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>>
    {
        let mut file = if let Ok(file) = OpenOptions::new()
            .write(true)
            .append(true)
            .open(file_name)
        {
            file
        } else {
            return Err("Error while opening the file to append the block".into());
        };

        let block_string = serde_json::to_string(block)?;
        file.write_all(format!("{}\n", block_string).as_bytes())?;

        Ok(())
    }
    
    pub fn init_first_block(&mut self) {
        self.blocks.push(Block::genesis());
    }

    pub fn add_block(&mut self, block: Block) {
        if !self.validate_block(&block) {
            println!("Invalid block: {:?}", block);
            return;
        }
        self.blocks.push(block);
    }

    pub fn get_blocks_by_indices_from_file(indices: Vec<u64>, file_name: &str) -> Option<Vec<Block>> {
        let file = if let Ok(file) = File::open(file_name) {
            file
        } else {
            println!("[LOAD BLOCKS FROM FILE] Error while opening the file");
            return None;
        };
        let reader = io::BufReader::new(file);

        let mut blocks = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            if indices.contains(&((i + 1) as u64)) {
                if let Ok(line) = line {
                    if let Ok(block) = serde_json::from_str(&line) {
                        blocks.push(block);
                    } else if let Err(e) = serde_json::from_str::<Block>(&line) {
                        println!("[LOAD BLOCKS FROM FILE] Error while parsing the block");
                        return None;
                    }
                } else {
                    println!("[LOAD BLOCKS FROM FILE] Error while reading the file");
                    return None;
                }
            }
        }

        Some(blocks)
    }

    pub fn get_last_n_blocks_from_file(n: usize, file_name: &str) -> Option<Vec<Block>> {
        let blockchain_length =
            if let Err(e) = Chain::get_blockchain_length(file_name) {
                println!("Error while getting last block from file: {}", e);
                0
            } else {
                Chain::get_blockchain_length(file_name).unwrap()
            };
        let mut last_n_blocks = None;

        if blockchain_length > 0 {
            let indices = if blockchain_length <= n {
                // Collect all indices into the vector
                (1..(blockchain_length as u64 + 1)).collect()
            } else {
                // Collect the range of indices into the vector
                ((blockchain_length as u64 - n as u64 + 1)..(blockchain_length as u64 + 1)).collect()
            };
            last_n_blocks = Chain::get_blocks_by_indices_from_file(indices, file_name);
        }

        last_n_blocks
    }

    pub fn get_range_of_blocks_from_file(start_idx: u64, end_idx: u64, file_name: &str) -> Option<Vec<Block>> {
        let blockchain_length =
            if let Err(e) = Chain::get_blockchain_length(file_name) {
                println!("Error while getting last block from file: {}", e);
                0
            } else {
                Chain::get_blockchain_length(file_name).unwrap()
            };
            
            if start_idx < 1 {
            println!("Start index must be greater than 0");
            return None;
        }
        if end_idx > blockchain_length as u64 + 1 {
            println!("End index must be less than or equal to the length of the blockchain");
            return None;
        }
        if end_idx < start_idx {
            println!("End index must be greater than or equal to start index");
            return None;
        }
        
        let mut blocks = None;
        if blockchain_length > 0 {
            let indices = if end_idx > blockchain_length as u64 {
                // Collect all indices into the vector
                (start_idx..(blockchain_length as u64 + 1)).collect()
            } else {
                // Collect the range of indices into the vector
                (start_idx..(end_idx + 1)).collect()
            };
            blocks = Chain::get_blocks_by_indices_from_file(indices, file_name);
        }

        blocks
    }

    pub fn load_block_from_file(block_idx: u64, file_name: &str) -> Option<Block> {
        // TODO: we assume that the file is not corrupted and that, for simplicity, every
        // block is on separate line. So to get ith block we simply read the ith line.
        let file = if let Ok(file) = File::open(file_name) {
            file
        } else {
            println!("[LOAD BLOCK FROM FILE] Error while opening the file");
            return None;
        };
        let reader = io::BufReader::new(file);

        // Read the file until reaching the desired element index
        for (i, line) in reader.lines().enumerate() {
            if i == (block_idx - 1) as usize {
                if let Ok(line) = line {
                    if let Ok(block) = serde_json::from_str(&line) {
                        return Some(block);
                    } else {
                        println!("[LOAD BLOCK FROM FILE] Error while parsing the block");
                        return None;
                    }
                } else {
                    println!("[LOAD BLOCK FROM FILE] Error while reading the file");
                    return None;
                }
            }
        }

        println!("[LOAD BLOCK FROM FILE] Unable to find the block with ID {}", block_idx);
        None
    }

    pub fn get_last_block(&self) -> Option<&Block> {
        self.blocks.last()
    }

    pub fn get_last_block_from_file(file_name: &str) -> Option<Block> {
        let blockchain_length =
            if let Err(e) = Chain::get_blockchain_length(file_name) {
                warn!("Error while getting last block from file: {}", e);
                0
            } else {
                Chain::get_blockchain_length(file_name).unwrap()
            };
        let mut last_block = None;

        if blockchain_length > 0 {
            last_block = if let Some(block) = Chain::load_block_from_file(
                blockchain_length as u64,
                file_name)
            {
                Some(block)
            } else {
                None
            }
        }

        last_block
    }

    pub fn remove_last_block(&mut self) {
        self.blocks.pop();
    }

    pub fn remove_last_block_from_file(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(file) = File::open(file_name){
            if file.metadata()?.len() == 0 {
                println!("The file is empty.");
                return Ok(());
            }
            let reader = io::BufReader::new(&file);
            let mut last_non_empty_line_pos: Option<u64> = None;
            let mut last_line_pos = 0;
            for line in reader.lines() {
                let line = line?;
                last_line_pos += line.len() as u64 + 1; // Add 1 for the line break

                if !line.is_empty() {
                    last_non_empty_line_pos = Some(last_line_pos);
                }
            }

            if let Some(pos) = last_non_empty_line_pos {
                // Truncate the file to remove the last non-empty line
                let file = OpenOptions::new().write(true).open(file_name)?;
                file.set_len(pos)?;
                println!("Last non-empty line removed successfully.");
            } else {
                println!("No non-empty line found.");
            }
        } else {
            println!("Error while removing last block from file");
        };

        Ok(())
    }

    pub fn choose_random_block_hashes(&self) -> Vec<String> {
        let hashes_to_choose = if self.blocks.len() < self.num_sidelinks {
            self.blocks.len()
        } else {
            self.num_sidelinks
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

    pub fn validate_chain_from_file(blockchain_filepath: &str) -> bool {
        if let Ok(chain) = Chain::load_from_file(blockchain_filepath) {
            chain.validate_chain()
        } else {
            println!("Error while loading the chain from file");
            false
        }
    }

    pub fn validate_chain(&self) -> bool {
        // Check if the chain is empty
        if self.blocks.is_empty() {
            println!("Verification of the chain failed. \
                The chain is empty.");
            return false;
        }

        // Check if the genesis block is correct
        if self.blocks[0] != Block::genesis() {
            println!("Verification of the chain failed. \
                The genesis block is incorrect.");
            return false;
        }

        // Check if the chain is continuous
        for i in 1..self.blocks.len() {
            if !self.validate_block(&self.blocks[i]) {
                println!("Verification of the chain failed. \
                    Block with ID {} is invalid.", i + 1);
                return false;
            }
        }

        true
    }

    fn validate_block_core(block: &Block,
        blockchain_filepath: Option<&str>,
        chain: Option<&Chain>,
        source: BlockValidationSource,
    ) -> bool
    {
        // Check if the block is the genesis block
        if block.idx == 1 {
            if *block != Block::genesis() {
                println!("Verification of the genesis block failed. \
                    Invalid data stored in the genesis block.");
                println!("Expected: {:?}\nActual: {:?}", Block::genesis(), block);
                return false;
            }
            return true;
        }

        let previous_block = match source {
            BlockValidationSource::File => {
                let block_from_file = Chain::load_block_from_file(
                    block.idx - 1,
                    blockchain_filepath.unwrap());
                block_from_file
            }
            BlockValidationSource::Chain => {
                if let Some(block) = chain.unwrap().blocks.get(block.idx as usize - 2) {
                    Some((*block).clone())
                } else {
                    None
                }
            }
        };

        if let Some(previous_block) = previous_block {
            // Check the correctness of ID of the block
            if block.idx != previous_block.idx + 1 {
                println!("Verification of block with ID {}. \
                    Invalid ID of the block; should be: {}",
                    block.idx, previous_block.idx + 1);
                return false;
            }

            // Check if the block is the next block in the chain
            let previous_block_hash = previous_block.hash();
            if block.previous_block_hash != previous_block_hash {
                println!("Verification of block with ID {}. \
                    Invalid hash of the previous block: stored: {:?}, actual hash: {:?}",
                    block.idx, block.previous_block_hash, previous_block_hash);
                return false;
            }

            let validation_sidelinks = block.derive_sidelink_indices();
            // Check if the number of hashes of previous blocks is correct
            if validation_sidelinks.len() != block.num_sidelinks {
                println!("Verification of block with ID {}. \
                    Invalid number of hashes of previous blocks: stored: {}, actual: {}",
                    block.idx, block.num_sidelinks, validation_sidelinks.len());
                return false;
            }

            // Check if the hashes of previous blocks are correct
            let sidelinked_blocks = match source {
                BlockValidationSource::File => {
                    Chain::get_blocks_by_indices_from_file(
                        validation_sidelinks,
                        blockchain_filepath.unwrap())
                }
                BlockValidationSource::Chain => {
                    let mut blocks = Vec::new();
                    for idx in validation_sidelinks {
                        if let Some(block) = chain.unwrap().blocks.get(idx as usize - 1) {
                            blocks.push((*block).clone());
                        } else {
                            println!("Was unable to get the block with ID {} from the chain. \
                                Verification of block with ID {} failed.", idx, block.idx);
                            return false;
                        }
                    }
                    Some(blocks)
                }
            };

            if let Some(sidelinked_blocks) = sidelinked_blocks {
                if sidelinked_blocks.len() > 0 {
                    for (i, block) in sidelinked_blocks.iter().enumerate() {
                        if block.hash() != block.validation_sidelinks[i] {
                            println!("Verification of block with ID {}. \
                                Invalid hash of the block with ID {}",
                                block.idx, block.idx);
                            return false;
                        }
                    }
                }
            } else {
                println!("Was unable to get the sidelinked blocks from {:?}. \
                    Verification of block with ID {} failed.", source, block.idx);
                return false;
            }

            // Check the proof of work
            let hash_result = pow::get_token_from_block(&block);
            let token = hash_result.as_slice();
            // println!("block.pow: {:?}", block.pow);
            // println!("block.previous_hash: {:?}", block.previous_block_hash);
            // println!("token: {:?}", token);
            // TODO: using the static value for now since the difficulty isn't rea;;y calculated
            if token.cmp(block.difficulty.as_slice()) != std::cmp::Ordering::Less {
                println!("Verification of block with ID {}. \
                    Invalid proof of work: {:?} >= {:?}",
                    block.idx, token, block.difficulty.as_slice());
                return false;
            }
        } else {
            println!("Was unable to get the last block of the chain from {:?}. \
                Verification of block with ID {} failed.", source, block.idx);
            return false;
        }

        true
    }

    pub fn validate_block_using_file(block: &Block, blockchain_filepath: &str) -> bool {
        Chain::validate_block_core(block,
            Some(blockchain_filepath),
            None,
            BlockValidationSource::File)
    }
    
    fn validate_block(&self, block: &Block) -> bool {
        Chain::validate_block_core(block,
            None,
            Some(self),
            BlockValidationSource::Chain)
    }
}
