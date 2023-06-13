use openssl::sha::sha256;
use rand::Rng;
use tokio::sync::mpsc;
use std::thread;

use crate::blockchain::block::Block;
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
fn prove_the_work(difficulty: &Vec<u8>, data: &str) -> String{
    // println!("Proving the work... (mining a block)");
    // Generate a random initial nonce so that the work of every node would not just be
    // a race of who can find the lowest nonce the fastest.
    let mut nonce = rand::thread_rng().gen::<u64>();
    let mut counter = 0;

    loop {
        let hash_result = sha256(&[data.as_bytes(), &nonce.to_be_bytes()].concat());
        let token = hash_result.as_ref();
        // Compare which one is smaller
        // println!("token: {:?}\ndifficulty: {:?}", token, difficulty);
        if token < difficulty.as_slice() {
            // println!("Found a valid nonce: {}", nonce);
            break;
        }
        if nonce % 10000000 == 0 {
            println!("Mining... Current nonce: {}.", nonce);
        }
        nonce += 1;
        counter += 1;
    }

    println!("Number of iterations: {}", counter);
    nonce.to_string()
}

// Mining function
async fn inifinite_pows(pow_tx: mpsc::UnboundedSender<String>,
    new_initial_data_rx: &mut mpsc::UnboundedReceiver<String>,
    difficulty: &Vec<u8>,
    data: &str)
{
    let mut data = data.to_string();
    let thread_id = thread::current().id();
    println!("inner miner thread ID: {:?}", thread_id);
    loop {
        let new_pow = prove_the_work(difficulty, &data);
        // println!("New proof of work: {}", new_pow);
        tokio::select! {
            Some(new_data) = new_initial_data_rx.recv() => {
                // If we mined a block but somebody mined it faster than our previous block is not
                // valid anymore and we need to mine a new block with new data
                data = new_data;
            }
            _ = tokio::task::yield_now() => {
                println!("Sending new proof of work via channel: {}", new_pow);
                if let Err(e) = pow_tx.send(new_pow) {
                    eprintln!("error sending new proof of work via channel, {}", e);
                }
            }
        }
    }
}

/*
    How this works:
        1. The mining task is spawned and it starts mining a block with the data of the last
            block in the chain.
        2. If a new block is mined, it is sent to the *main* function via the channel.
        3. If a new block is added to the chain, the mining task is notified and it starts
            mining a new block with the data of the new last block in the chain. This information
            is received via new_last_block_rx channel.
        4. Since mining is an infinite loop, to update it about new data used for mining we
            send it via new_mining_data_tx channel (i.e. we compute the new data needed to compute
            the nonce and then send it).
 */
pub async fn mine_blocks(new_mined_block_tx: &mpsc::UnboundedSender<Block>,
    new_last_block_rx: &mut mpsc::UnboundedReceiver<Block>,
    difficulty: &Vec<u8>,
    last_block: Block
) {
    let (pow_tx, mut pow_rx) = mpsc::unbounded_channel();
    let (mining_data_tx, mut mining_data_rx) = mpsc::unbounded_channel();
    // TODO: this should use sidelinks (i.e. generate random indices of blocks using this hash
    // and then calculate their hashes and concatenate them with this hash and use it as data)
    let main_hash = last_block.hash();
    let mut mining_data = main_hash.clone();
    
    // Mining task, create a copy of the difficulty vector
    let difficulty = difficulty.clone();
    
    let _ = tokio::spawn(async move {
        inifinite_pows(pow_tx, &mut mining_data_rx, &difficulty, mining_data.as_str()).await;
    });
    
    let thread_id = thread::current().id();
    println!("outer miner thread ID: {:?}", thread_id);
    
    loop {
        tokio::select! {
            Some(new_last_block) = new_last_block_rx.recv() => {
                // TODO: this should use sidelinks (i.e. generate random indices of blocks using this hash
                // and then calculate their hashes and concatenate them with this hash and use it as data)
                mining_data = new_last_block.hash();
                if let Err(e) = mining_data_tx.send(mining_data.clone()) {
                    eprintln!("error sending new mining data via channel, {}", e);
                };
            },
            Some(pow) = pow_rx.recv() => {
                println!("Received New proof of work: {}", pow);
                let mined_block = Block::new(last_block.idx + 1,
                    // TODO: second computation of the hash of the last block
                    last_block.hash(),
                    Vec::new(),
                    pow,
                    Vec::new());
    
                if let Err(e) = new_mined_block_tx.send(mined_block) {
                    eprintln!("error sending new mined block via channel, {}", e);
                };
            },
            // _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
            //     println!("Mining...");
            // }
        }
    }    
}