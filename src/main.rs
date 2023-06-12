mod blockchain;
mod utils;
mod network;
mod blockchain_io;

use crate::utils::find_my_hashrate;
use crate::network::event::{InternalResponse, handle_incoming_event};
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
    let (internal_tx, mut internal_rx) = mpsc::unbounded_channel();
    // Clear the screen every 10 events
    let mut event_counter = 0;
    print_cmd_options();
    loop {
        tokio::select! {
            // TODO: create enum and hanlde every case using that enum to avoid huge code chunks
            // executed within select
            cmd_line = stdin.next_line() => {
                let line = cmd_line.expect("can get line").expect("can read line from stdin");
                println!("Received user input: {:?}", line);
                process_cmd(line, &mut swarm, &local_peer_id, blockchain_filepath.as_str());
            }
            internal_response = internal_rx.recv() => {
                let internal_response: InternalResponse = internal_response.expect("can get internal response");
                println!("Received internal response: {:?}", internal_response);
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
                    propagation_source: _peer_id,
                    // Random number incremented by 1 with each message
                    message_id: _id,
                    message,
                })) => {
                    // Decerialize the message
                    let data = String::from_utf8_lossy(&message.data).to_string();
                    handle_incoming_event(&data,
                        &local_peer_id,
                        &internal_tx,
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
