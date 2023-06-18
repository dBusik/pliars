mod blockchain;
mod utils;
mod network;
mod blockchain_io;

use crate::network::{event::{NetworkEvent, CHAIN_INITIALIZATION_DONE}, event_handling};
use crate::network::behaviour::{BlockchainBehaviour, BlockchainBehaviourEvent, Topics};
use crate::blockchain_io::{process_simple_cmd, print_cmd_options};
use blockchain::{
    pow,
    chain::{Chain, DIFFICULTY_VALUE, DEFAULT_DIFFICULTY_IN_SECONDS, DEFAULT_NUM_OF_SIDELINKS},
    block::Record,
};

use libp2p::gossipsub::Behaviour;
use tokio::{self, sync::mpsc, io::AsyncBufReadExt};
use std::{time::Duration};
use libp2p::core::{upgrade};
use libp2p::futures::StreamExt;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::{identity, Transport, noise, tcp, PeerId, yamux, gossipsub, mdns};
use std::thread;
use log::{error, info, warn};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    pretty_env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let blockchain_filepath = format!("./blockchain_storage_{local_peer_id}.json");

    info!("Starting the node... PEER ID: {local_peer_id}");
    info!("[PEER ID {}] blockchain filepath: {}", local_peer_id, blockchain_filepath);

    // Set encrypted DNS-enabled TCP transport over yamux multiplexing
    let tcp_transport = tcp::tokio::Transport::default()
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(&local_key).expect("signing libp2p-noise static keypair"))
        .multiplex(yamux::Config::default())
        .boxed();

    // Set a gossipsub configuration
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
        .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
        // .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
        .build()
        .expect("Valid gossipsub configuration");

    // Build a gossipsub network behaviour
    let mut gossipsub: Behaviour = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key),
        gossipsub_config,
    ).expect("Correct network behaviour configuration");

    // Create topics and subscribe to them
    for topic in [Topics::Block, Topics::Chain, Topics::Message, Topics::Record].iter() {
        let topic = gossipsub::IdentTopic::new(topic.to_string());
        gossipsub.subscribe(&topic).expect("Subscribed to topic");
        info!("Subscribed to topic: {:?}", topic);
    }

    // Create a swarm to manage peers and events
    let mut swarm = {
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;
        let behaviour = BlockchainBehaviour { gossipsub, mdns };
        SwarmBuilder::with_tokio_executor(tcp_transport, behaviour, local_peer_id).build()
    };
    
    swarm.listen_on("/ip4/0.0.0.0/tcp/0"
        .parse()
        .expect("Able to get a local socket")
    ).expect("Swarm can be started");
    info!("Listening. Network info {:?}", swarm.network_info());

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    // Channels for new mined blocks
    let (new_mined_block_tx, mut new_mined_block_rx) = mpsc::unbounded_channel();
    // Channels to inform the miner about new last block of the chain
    let (new_last_block_tx, mut new_last_block_rx) = mpsc::unbounded_channel();
    // Channel to send new records to the minder thread so that they will be appended to the
    // block being mined
    let (new_record_tx, mut new_record_rx) = mpsc::unbounded_channel();

    // Clear the screen every 10 events
    let mut event_counter = 0;
    print_cmd_options();

    // Spawn the block mining task
    let hashrate: f64 = utils::find_my_hashrate() as f64;
    let difficulty = utils::difficulty_from_secs(DEFAULT_DIFFICULTY_IN_SECONDS, hashrate);
    info!("[SYSTEM] Starting the mining task with difficulty: {:?}", difficulty);
    
    // Dispatch the mine_blocks function
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .worker_threads(3) // Set the number of worker threads
        .build()
        .unwrap();

    let fpath_copy = blockchain_filepath.clone();
    let difficulty_copy = difficulty.clone();
    runtime.spawn(async move {
        pow::mine_blocks(&new_mined_block_tx,
            &mut new_last_block_rx,
            &mut new_record_rx,
            &difficulty_copy,
            &fpath_copy).await;
    });

    let thread_id = thread::current().id();
    info!("[SYSTEM] Main function thread ID: {:?}", thread_id);

    loop {
        info!("Waiting for event...");
        tokio::select! {
            // TODO: create enum and hanlde every case using that enum to avoid huge code chunks
            // executed within select
            Some(mined_block) = new_mined_block_rx.recv() => {
                // println!("[NEW_BLOCK_MINED] Received mined block: {:?}", mined_block);
                info!("[NEW_BLOCK_MINED] Received mined block; idx = {}", mined_block.idx);
                let block_proposal = NetworkEvent::BlockProposal(mined_block);
                block_proposal.send(&mut swarm);
            }
            cmd_line = stdin.next_line() => {
                let line = cmd_line.expect("can get line").expect("can read line from stdin");
                info!("[NEW_USER_INPUT] {:?}", line);
                // If line is "init" then process the event here, otherwise use
                // the process_cmd function
                if line.starts_with("init") {
                    info!("Init received");
                    if unsafe { CHAIN_INITIALIZATION_DONE } {
                        warn!("Blockchain exists. Not initializing the blockchain");
                        // Jump out of the match and continue the loop
                        continue;
                    }
                    // Safe alternative to the above code (not too compelling though)
                    // if std::path::Path::new(blockchain_file).exists() {
                    //     println!("Blockchain exists. Not initializing the blockchain");
                    //     return;
                    // }

                    let hashrate: f64 = utils::find_my_hashrate() as f64;
                    info!("My hashrate: {}", hashrate);

                    let mut user_input = line.split_whitespace();
    
                    // TODO: user input difficulty is ignored since the code is not ready for
                    // dynamic difficulty adjustment
                    // let difficulty_in_secs = if let Some(difficulty) = user_input.next() {
                    //     let diff_val = difficulty.parse()
                    //         .expect("can parse difficulty");
                    //     diff_val
                    // } else {
                    //     DEFAULT_DIFFICULTY_IN_SECONDS
                    // };

                    let num_sidelinks = if let Some(sidelinks_num) = user_input.next() {
                        let sidel_val = if let Ok(sidel_val) = sidelinks_num.parse::<usize>() {
                            sidel_val
                        } else {
                            DEFAULT_NUM_OF_SIDELINKS
                        };
                        sidel_val
                    } else {
                        DEFAULT_NUM_OF_SIDELINKS
                    };

                    // Difficulty should be such that number of seconds to mine a block is equal to
                    // a value given by the user or DEFAULT_DIFFICULTY_IN_SECONDS if the user did not
                    // provide any value.
                    // Since max hash value for sha256 is 2^256-1, we can calculate the difficulty
                    // number later used for comparison with hashes as
                    //     2^256-1 / (<difficulty_in_seconds>> * <hashrate of the network>)

                    // TODO: user input difficulty is ignored since the code is not ready for
                    // dynamic difficulty adjustment                    
                    // let difficulty = utils::difficulty_from_secs(difficulty_in_secs, hashrate);
                    let mut blockchain = Chain::new(num_sidelinks);
                    blockchain.init_first_block();
                    // blockchain.add_block(block::Block::genesis());
                    
                    info!("Saving blockchain to file {}", blockchain_filepath);
                    if blockchain.save_blockchain_to_file(&blockchain_filepath).is_err() {
                        error!("Error while saving blockchain to file, cancelling the init event");
                    }

                    // TODO: user input difficulty is ignored since the code is not ready for
                    // dynamic difficulty adjustment        
                    // info!("Trying to send to other peers Init event with difficulty: \
                    //     {:?}[secs] (or {:?} as u8 vector) and number of sidelinks: {:?}",
                    //     DEFAULT_DIFFICULTY_IN_SECONDS, difficulty, num_sidelinks);

                    unsafe {
                        CHAIN_INITIALIZATION_DONE = true;
                        DIFFICULTY_VALUE = difficulty.clone();
                        info!("Difficulty set to {:?}", DIFFICULTY_VALUE);
                    }
                    // Send new last block to mining thread
                    new_last_block_tx.send(blockchain.get_last_block().unwrap().clone()).unwrap();
                    NetworkEvent::InitUsingChain(blockchain).send(&mut swarm);             
                } else if line.starts_with("rec") {
                    info!("rec received");
                    let mut user_input = line.split_whitespace();
                    // Second word is record data
                    let record_data = if let Some(data) = user_input.nth(1) {
                        data.to_string()
                    } else {
                        warn!("No record data provided");
                        continue;
                    };

                    let new_record = Record::new(record_data.clone(), 
                        local_peer_id.to_string());
                    let new_record_clone = new_record.clone();
                    if let Err(e) = new_record_tx.send(new_record) {
                        error!("Error sending new record to the mining thread: {}", e);
                    } else {
                        info!("Sending new record with data {:?} other peers", new_record_clone);
                        NetworkEvent::NewRecord(new_record_clone).send(&mut swarm);
                    }
                } else {
                    process_simple_cmd(line, &mut swarm, &local_peer_id, blockchain_filepath.as_str());
                }
            }
            network_event = swarm.select_next_some() => match network_event {
                SwarmEvent::Behaviour(BlockchainBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        info!("[NETWORK] mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(BlockchainBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        info!("[NETWORK] mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                // Do not confuse this message with NetworkEvent defined by this crate.
                // gossipsub::Event::Message is pre-defined by the libp2p-gossipsub crate.
                SwarmEvent::Behaviour(BlockchainBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    // Derived from the key
                    propagation_source: peer_id,
                    // Random number incremented by 1 with each message
                    message_id: _id,
                    message,
                })) => {
                    // Decerialize the message
                    let data = String::from_utf8_lossy(&message.data).to_string();
                    // info!("[NETWORK] Received message: {:?}", data);
                    event_handling::handle_incoming_network_event(&data,
                        &local_peer_id,
                        &peer_id,
                        &mut swarm,
                        &new_last_block_tx,
                        &new_record_tx,
                        &blockchain_filepath);
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("[NETWORK] Local node is listening on {address}");
                }
                _ => {
                    // info!("[NETWORK] Unhandled swarm event: {:?}", network_event);
                    info!("[NETWORK] Unhandled swarm event");
                }
            }
        }

        event_counter += 1;
        if event_counter % 10 == 0 {
            print!("Clearing the screen.\n{}[2J", 27 as char);
            event_counter = 0;
            print_cmd_options();
        }
    }
}
