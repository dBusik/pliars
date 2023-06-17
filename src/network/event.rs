use serde::{Serialize, Deserialize};
use libp2p::gossipsub;

use crate::blockchain::block;
use crate::blockchain::{
    block::Block,
    chain::{Chain},
};
use crate::BlockchainBehaviour;
use crate::utils;
use super::CHAIN_INITIALIZED;
use crate::network::behaviour::Topics;

pub const DEFAULT_DIFFICULTY_IN_SECONDS: f64 = 30.0;
pub const DEFAULT_NUM_OF_SIDELINKS: usize = 5;

#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkEvent {
    InitFromUserIo{ difficulty: Option<f64>, num_sidelinks: Option<usize> },
    InitUsingChain(Chain),
    BlockProposal(Block),
    RemoteChainRequest { asked_peer_id: String },
    RemoteChainResponse { chain_from_sender: Chain, chain_receiver: String },
    // Messages are more of a gimmick and can be exchanged between nodes along with
    // the blocks and chains. They do not impact the blockchain in any way.
    Message { message: String, from_peer_id: String },
    StartMining,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum InternalResponse {
    BlockResponse(Block),
    ChainResponse(Chain),
}

impl NetworkEvent {
    pub fn _to_string(&self) -> String {
        serde_json::to_string(&self).expect("can serialize network event")
    }

    pub fn from_string(string: &str) -> NetworkEvent {
        serde_json::from_str(&string).expect("can deserialize network event")
    }

    pub fn send(&self, swarm: &mut libp2p::Swarm<BlockchainBehaviour>, blockchain_file: &str) {
        match self {
            NetworkEvent::InitFromUserIo {difficulty, num_sidelinks } => {
                if unsafe { CHAIN_INITIALIZED } {
                    println!("Blockchain exists. Not initializing the blockchain");
                    return;
                }

                // Safe alternative to the above code (not too compelling though)
                // if std::path::Path::new(blockchain_file).exists() {
                //     println!("Blockchain exists. Not initializing the blockchain");
                //     return;
                // }

                let hashrate: f64 = utils::find_my_hashrate() as f64;
                println!("My hashrate: {}", hashrate);
                let difficulty_in_secs: f64 = difficulty.unwrap_or(DEFAULT_DIFFICULTY_IN_SECONDS);
                let num_side_links: usize = num_sidelinks.unwrap_or(DEFAULT_NUM_OF_SIDELINKS);
                // Difficulty should be such that number of seconds to mine a block is equal to
                // a value given by the user or DEFAULT_DIFFICULTY_IN_SECONDS if the user did not
                // provide any value.
                // Since max hash value for sha256 is 2^256-1, we can calculate the difficulty
                // number later used for comparison with hashes as
                //     2^256-1 / (<difficulty_in_seconds>> * <hashrate of the network>)
                let difficulty = utils::difficulty_from_secs(difficulty_in_secs, hashrate);
                let mut blockchain = Chain::new(difficulty.clone(), num_side_links);
                blockchain.init_first_block();
                blockchain.add_block(block::Block::genesis());
                
                if blockchain.save_blockchain_to_file(blockchain_file).is_err() {
                    println!("Error while saving blockchain to file, cancelling the init event");
                    return;
                }

                println!("Trying to send to other peers Init event with difficulty: \
                    {:?}[secs] (or {:?} as u8 vector) and number of sidelinks: {:?}",
                    difficulty_in_secs, difficulty, num_sidelinks);

                unsafe { CHAIN_INITIALIZED = true; }
                let pending_event = NetworkEvent::InitUsingChain(blockchain);
                send_network_event(pending_event, swarm);
            },
            NetworkEvent::StartMining => {
                println!("Trying to send StartMining event");
                let pending_event = NetworkEvent::StartMining;
                send_network_event(pending_event, swarm);
            },
            NetworkEvent::InitUsingChain(chain) => {
                println!("Trying to send InitUsingChain event");
                let pending_event = NetworkEvent::InitUsingChain(chain.clone());
                send_network_event(pending_event, swarm);
            },
            NetworkEvent::BlockProposal(_) => {
                println!("Trying to send BlockProposal event");
            },
            NetworkEvent::RemoteChainRequest { .. } => {
                println!("Trying to send RemoteChainRequest event");
            },
            NetworkEvent::RemoteChainResponse { .. } => {
                println!("Trying to send RemoteChainResponse event");
            },
            NetworkEvent::Message { message, from_peer_id } => {
                println!("Trying to send Message event");
                let message = NetworkEvent::Message {
                    message: message.clone(),
                    from_peer_id: from_peer_id.clone(),
                };
                send_network_event(message, swarm);
            },
        }
    }
}

pub fn send_network_event(
    pending_event: NetworkEvent,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
) {
    let topic = match pending_event {
        NetworkEvent::InitUsingChain(_) => Topics::Chain,
        NetworkEvent::BlockProposal(_) => Topics::Block,
        NetworkEvent::RemoteChainRequest { .. } => Topics::Chain,
        NetworkEvent::RemoteChainResponse { .. } => Topics::Chain,
        NetworkEvent::Message { .. } => Topics::Message,
        // If mining or user io event is received, do not send it to other peers
        _ => {
            println!("Received local event: {:?}; local events are not meant to be sent\
                to other peers", pending_event);
            return;
        },
    };

    if let Err(e) = swarm.behaviour_mut().gossipsub.publish(
        gossipsub::IdentTopic::new(topic.to_string()),
        serde_json::to_vec(&pending_event).expect("can serialize message"))
    {
        if let libp2p::gossipsub::PublishError::InsufficientPeers = e {
            println!("No peers to share event with to :(");
        } else {
            panic!("Error while publishing message: {:?}", e);
        }
    } else {
        println!("Event sent successfully: {:?}", pending_event);
    }
}
