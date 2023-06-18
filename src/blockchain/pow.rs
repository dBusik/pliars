use openssl::sha::sha256;
use rand::Rng;
use tokio::sync::mpsc;
use core::panic;
use std::thread;

use crate::blockchain::{block::Block, chain::Chain};

pub fn get_token_from_block(block: &Block) -> [u8; 32] {
    sha256(&[block.previous_block_hash.as_bytes(),
        // &block.difficulty,
        &(block.pow.parse::<u64>().unwrap().to_be_bytes())].concat())
}

pub fn get_new_token(new_block_so_far: &Block, nonce: u64) -> [u8; 32] {
    sha256(&[new_block_so_far.previous_block_hash.as_bytes(),
        // &new_block_so_far.difficulty,
        &nonce.to_be_bytes()].concat())
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
fn prove_the_work(difficulty: &Vec<u8>,
    last_block: &Block,
    new_last_block_rx: &mut mpsc::UnboundedReceiver<Block>
) -> Block {
    // println!("Proving the work... (mining a block)");
    // Generate a random initial nonce so that the work of every node would not just be
    // a race of who can find the lowest nonce the fastest.
    let mut nonce = rand::thread_rng().gen::<u64>();
    let mut counter = 0;

    let mut new_block = Block::new(
        last_block.idx + 1,
        last_block.hash(),
        Vec::new(),
        "".to_string(),
        Vec::new(),
        difficulty.clone(),
    );

    loop {
        let hash_result = get_new_token(&new_block, nonce);
        let token = hash_result.as_ref();
        // Compare which one is smaller
        // println!("token: {:?}\ndifficulty: {:?}", token, difficulty);
        if token < difficulty.as_slice() {
            // println!("Found a valid nonce: {}", nonce);
            break;
        }
        if nonce % 10000000 == 0 {
            // Check if something came through the channel
            if let Ok(new_last_block) = new_last_block_rx.try_recv() {
                // println!("New last block received: {:?}", new_last_block);
                // If something came through the channel, discard the current block and start
                // mining a new block with the data of the new last block
                nonce = rand::thread_rng().gen::<u64>();
                new_block.previous_block_hash = new_last_block.hash();
                counter = 0;
                println!("New last block with hash {} received. Discarding the current block and\
                    starting mining a new block with the data of the new last block.",
                    new_block.previous_block_hash);
                continue;
            }
            println!("Mining... Current nonce: {}.", nonce);
        }
        nonce += 1;
        counter += 1;
    }

    // println!("Number of iterations: {}", counter);
    new_block.pow = nonce.to_string();
    new_block
}

/*
    How this works:
        1. The mining task is spawned and it starts mining a block with the data of the last
            block in the chain.
        2. If a new block is mined, it is sent to the *main* function via the channel.
        3. If a new block is added to the chain, the mining task is notified and after mining the
            previous block it discards it and starts mining a new block with the data of the new
            last block in the chain.
 */
pub async fn mine_blocks(new_mined_block_tx: &mpsc::UnboundedSender<Block>,
    new_last_block_rx: &mut mpsc::UnboundedReceiver<Block>,
    difficulty: &Vec<u8>,
    blockchain_filepath: &str
) {
    let mut last_block = if let Some(block) =
        Chain::get_last_block_from_file(blockchain_filepath)
    {
        block
    } else {
        // Lock the thread and wait on the channel
        println!("[MINER]: Waiting for chain initialization...\
            (either get somebody's chain or use the init command)");
        new_last_block_rx.recv().await.unwrap()
    };

    // Mining task, create a copy of the difficulty vector
    let difficulty = difficulty.clone();

    let thread_id = thread::current().id();
    println!("Miner starting thread ID: {:?}", thread_id);

    loop {
        let mined_block = prove_the_work(&difficulty, &last_block, new_last_block_rx);
        // println!("New proof of work: {}", new_pow);
        tokio::select! {
            Some(new_last_block) =  new_last_block_rx.recv() => {
                // If we mined a block but somebody mined it faster than our previous block is not
                // valid anymore and we need to mine a new block with new data
                last_block = new_last_block;
            }
            _ = tokio::task::yield_now() => {
                println!("Sending new block with such proof of work via channel: {}", mined_block.pow);
                // TODO: this should use sidelinks (i.e. generate random indices of blocks using this hash
                // and then calculate their hashes and concatenate them with this hash and use it as data)

                // println!("Old block: {:?}", last_block);
                // println!("New block: {:?}", mined_block);
                let new_last_block = mined_block.clone();
                if let Err(e) = Chain::append_block_to_file(&mined_block, blockchain_filepath) {
                    println!("Error appending block to file. Block will be discarded: {}.", e);
                } else {
                    println!("Block appended to file.");
                    if let Err(e) = new_mined_block_tx.send(mined_block) {
                        println!("Error sending new mined block via channel, {}", e);
                        if let Err(e) = Chain::remove_last_block_from_file(blockchain_filepath) {
                            println!("Tried to remove last block from the file due to
                                usuccessful broadcast of the new block but error occured: {}", e);
                        } else {
                            println!("Last block removed from file since broadcast of the block\
                                failed.");
                        }
                    } else {
                        println!("Sent new mined block via channel");
                        last_block = new_last_block;
                    }
                }
            }
            // _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
            //     println!("Mining...");
            // }
        }
    }
}