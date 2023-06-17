mod blockchain;
mod utils;
mod network;
mod blockchain_io;

use crate::utils::find_my_hashrate;
use crate::network::event_handling;
use crate::network::behaviour::{BlockchainBehaviour, BlockchainBehaviourEvent, Topics};
use crate::blockchain_io::{process_cmd, print_cmd_options};

use libp2p::gossipsub::Behaviour;
use log::info;
use tokio::{self, sync::mpsc, io::AsyncBufReadExt};
use std::{time::Duration};
use libp2p::core::{upgrade};
use libp2p::futures::StreamExt;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::{identity, Transport, noise, tcp, PeerId, yamux, gossipsub, mdns};
use rug;
use std::thread;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    pretty_env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let blockchain_filepath = format!("./blockchain_storage_{local_peer_id}.json");

    info!("Starting the node... PEER ID: {local_peer_id}");
    info!("[PEER ID {}], My hashrate: {} hashes/s", local_peer_id, find_my_hashrate());
    
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
    for topic in [Topics::Block, Topics::Chain, Topics::Message].iter() {
        let topic = gossipsub::IdentTopic::new(topic.to_string());
        gossipsub.subscribe(&topic).expect("Subscribed to topic");
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
    // let (new_mined_block_tx, mut new_mined_block_rx) = mpsc::unbounded_channel();
    // // Channels to inform the miner about new last block of the chain
    // let (new_last_block_tx, mut new_last_block_rx) = mpsc::unbounded_channel();
    
    // Clear the screen every 10 events
    let mut event_counter = 0;
    print_cmd_options();

    // Spawn the block mining task
    let network_difficulty_secs: f64 = 6.0;
    let hashrate: f64 = find_my_hashrate() as f64;
    let difficulty = (2.0f64.powi(256) - 1.0) / (network_difficulty_secs * hashrate);
    let difficulty = rug::Float::with_val(256, difficulty);
    let difficulty = difficulty.trunc().to_integer().unwrap();
    println!("Difficulty: {:?}", difficulty);
    let mut difficulty = difficulty.to_digits::<u8>(rug::integer::Order::MsfBe);
    while difficulty.len() < 32 {
        // Pad the difficulty with zeros if it is shorter that the length of the ouput
        // of the hash function (which in this case is 256 bits since we use sha256)
        difficulty.insert(0, 0);
    }
    println!("Starting the mining task with difficulty: {:?}", difficulty);
    
    // Dispatch the mine_blocks function
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .worker_threads(3) // Set the number of worker threads
        .build()
        .unwrap();

    // runtime.spawn(async move {
    //     pow::mine_blocks(&new_mined_block_tx, &mut new_last_block_rx, &difficulty, Block::genesis()).await;
    // });

    let thread_id = thread::current().id();
    println!("main function thread ID: {:?}", thread_id);

    loop {
        println!("Waiting for event...");
        tokio::select! {
            // TODO: create enum and hanlde every case using that enum to avoid huge code chunks
            // executed within select
            // Some(mined_block) = new_mined_block_rx.recv() => {
            //     println!("Received mined block: {:?}", mined_block);
            //     // let pending_event = BlockchainBehaviourEvent::BlockProposal(mined_block);
            //     // let topic = Topics::Block;
            //     // if let Err(e) = swarm.behaviour_mut().gossipsub.publish(
            //     //     gossipsub::IdentTopic::new(topic.to_string()),
            //     //     serde_json::to_vec(&pending_event).expect("can serialize message"))
            //     // {
            //     //     if let libp2p::gossipsub::PublishError::InsufficientPeers = e {
            //     //         println!("No peers to share event with to :(");
            //     //     } else {
            //     //         panic!("Error while publishing message: {:?}", e);
            //     //     }
            //     // }
            // }
            cmd_line = stdin.next_line() => {
                let line = cmd_line.expect("can get line").expect("can read line from stdin");
                println!("Received user input: {:?}", line);
                process_cmd(line, &mut swarm, &local_peer_id, blockchain_filepath.as_str());
            }
            network_event = swarm.select_next_some() => match network_event {
                SwarmEvent::Behaviour(BlockchainBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(BlockchainBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
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
                    event_handling::handle_incoming_network_event(&data,
                        &local_peer_id,
                        &peer_id,
                        &mut swarm,
                        blockchain_filepath.as_str());
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                _ => {
                    println!("Unhandled swarm event: {:?}", network_event);
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
