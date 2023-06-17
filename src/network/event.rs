use std::num;

use crate::blockchain::block;
use crate::blockchain::{
    block::Block,
    chain::{Chain, ChainType}
};
use crate::BlockchainBehaviour;
use crate::utils;
use crate::network::behaviour::Topics;

use serde::{Serialize, Deserialize};
use libp2p::gossipsub;

pub const DEFAULT_DIFFICULTY_IN_SECONDS: f64 = 10.0;
pub const DEFAULT_NUM_OF_SIDELINKS: usize = 5;

#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkEvent {
    Init{ difficulty: Option<f64>, num_sidelinks: Option<usize> },
    InitUsingChain(Chain),
    BlockProposal(Block),
    RemoteChainRequest { asked_peer_id: String },
    RemoteChainResponse { remote_chain: Chain, chain_receiver: String },
    // Messages are more of a gimmick and can be exchanged between nodes along with
    // the blocks and chains. They do not impact the blockchain in any way.
    Message { message: String, from_peer_id: String },
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
}

fn send_network_event(
    pending_event: NetworkEvent,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
) {
    let topic = match pending_event {
        NetworkEvent::Init { .. } => Topics::Chain,
        NetworkEvent::InitUsingChain(_) => Topics::Chain,
        NetworkEvent::BlockProposal(_) => Topics::Block,
        NetworkEvent::RemoteChainRequest { .. } => Topics::Chain,
        NetworkEvent::RemoteChainResponse { .. } => Topics::Chain,
        NetworkEvent::Message { .. } => Topics::Message
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
    }
}

impl NetworkEvent {
    pub fn send(&self, swarm: &mut libp2p::Swarm<BlockchainBehaviour>, blockchain_file: &str) {
        match self {
            NetworkEvent::Init {difficulty, num_sidelinks } => {
                if std::path::Path::new(blockchain_file).exists() {
                    println!("Blockchain exists. Not initializing the blockchain");
                    return;
                }
                println!("Sending to other peers Init event with difficulty:
                    {:?} and number of sidelinks: {:?}", difficulty, num_sidelinks);
                let network_difficulty_secs: f64 = difficulty.unwrap_or(DEFAULT_DIFFICULTY_IN_SECONDS);
                let hashrate: f64 = utils::find_my_hashrate() as f64;
                println!("My hashrate: {}", hashrate);
                // Difficulty should be such that number of seconds to mine a block is 10
                // Since max hash value for sha256 is 2^256-1, we can calculate the difficulty
                // as 2^256-1 / (10 * hashrate)
                let difficulty = (2.0f64.powi(256) - 1.0) / (network_difficulty_secs * hashrate);
                let num_side_links: usize = num_sidelinks.unwrap_or(DEFAULT_NUM_OF_SIDELINKS);
                
                let mut blockchain = Chain::new(difficulty, num_side_links);
                blockchain.init_first_block();
                blockchain.add_block(block::Block::genesis());

                if blockchain.save_blockchain_to_file(blockchain_file).is_err() {
                    println!("Error while saving blockchain to file");
                }

                println!("Sending Init event with blockchain[difficulty: {difficulty},
                    sidelinks: {num_side_links}]: {:?}", blockchain);

                let pending_event = NetworkEvent::InitUsingChain(blockchain);
                send_network_event(pending_event, swarm);
            },
            NetworkEvent::InitUsingChain(chain) => {
                println!("Sending InitUsingChain event");
                let pending_event = NetworkEvent::InitUsingChain(chain.clone());
                send_network_event(pending_event, swarm);
            },
            NetworkEvent::BlockProposal(_) => {
                println!("Sending BlockProposal event");
            },
            NetworkEvent::RemoteChainRequest { .. } => {
                println!("Sending RemoteChainRequest event");
            },
            NetworkEvent::RemoteChainResponse { .. } => {
                println!("Sending RemoteChainResponse event");
            },
            NetworkEvent::Message { message, from_peer_id } => {
                println!("Sending Message event");
                let message = NetworkEvent::Message {
                    message: message.clone(),
                    from_peer_id: from_peer_id.clone(),
                };
                send_network_event(message, swarm);
            },
        }
    }
}

fn handle_init_using_chain_event(chain: Chain, swarm: &mut libp2p::Swarm<BlockchainBehaviour>, blockchain_file: &str) {
    // println!("Received Init event");
    // let hashrate = utils::find_my_hashrate();
    // // Difficulty should be such that number of seconds to mine a block is 10
    // // Since max hash value for sha256 is 2^256-1, we can calculate the difficulty
    // // as 2^256-1 / (10 * hashrate)
    // // let difficulty = 
    // // let blockchain = Chain::new(, num_side_links)
    println!("Received InitUsingChain event: {:?}", chain);
    if chain.save_blockchain_to_file(blockchain_file).is_err() {
        println!("Error while saving blockchain to file");
    }
}

pub fn handle_incoming_network_event(event_data: &String,
    local_peer_id: &libp2p::PeerId,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
    blockchain_file: &str,
) {
    let event = NetworkEvent::from_string(event_data);
    match event {
        NetworkEvent::InitUsingChain(chain) => {
            handle_init_using_chain_event(chain, swarm, blockchain_file);
        }
        NetworkEvent::BlockProposal(block) => {
            println!("Received BlockProposal event: {:?}", block);
        }
        NetworkEvent::RemoteChainRequest { asked_peer_id } => {
            println!("Received RemoteChainRequest event: {:?}", asked_peer_id);
            if asked_peer_id == local_peer_id.to_string() {
                println!("Sending local chain to {}", asked_peer_id);
                // Check if chain is ok and ignore if not
                if let Ok(local_chain) = Chain::load_from_file(blockchain_file) {
                    let event = NetworkEvent::RemoteChainResponse {
                        remote_chain: local_chain,
                        chain_receiver: local_peer_id.to_string(),
                    };
                    send_network_event(event, swarm);
                } else {
                    println!("Chain is not valid. Ignoring RemoteChainRequest event");
                };
            }
        }
        NetworkEvent::RemoteChainResponse { remote_chain, chain_receiver } => {
            println!("Received RemoteChainResponse event: {:?} from {:?}", remote_chain, chain_receiver);
            if chain_receiver == local_peer_id.to_string() {
                println!("RemoteChainResponse is meant for {}, which is me", chain_receiver);
                // Compare the received chain with the local chain and choose the one with
                // the highest difficulty
                if let Ok(mut chain) = Chain::load_from_file(blockchain_file) {
                    let winner_chain_type = chain.choose_longest_chain(&remote_chain);
                    if winner_chain_type == ChainType::Remote {
                        if chain.save_blockchain_to_file(blockchain_file).is_err() {
                            println!("Error while saving remote chain to file");
                        }
                    }
                } else {
                    println!("Local chain is not valid. Veryfiyng remote chain and saving it as local chain");
                    if remote_chain.validate_chain() {
                        if remote_chain.save_blockchain_to_file(blockchain_file).is_err() {
                            println!("Error while saving remote chain to file");
                        }
                    } else {
                        println!("Remote chain is not valid. Ignoring RemoteChainResponse event");
                    }
                };
            }
        }
        NetworkEvent::Message { message, from_peer_id } => {
            println!("Received Message event: {:?} from {:?}", message, from_peer_id);
        }
        NetworkEvent::Init { difficulty, num_sidelinks } => {
            println!("Received Init event with difficulty: {:?} and number of sidelinks: {:?}", difficulty, num_sidelinks);
            handle_init_using_chain_event(
                Chain::new(difficulty.unwrap(), num_sidelinks.unwrap()),
                swarm,
                blockchain_file);
        }
    }
}
