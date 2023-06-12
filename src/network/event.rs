use crate::blockchain::block;
use crate::blockchain::{
    block::Block,
    chain::Chain,
};
use crate::BlockchainBehaviour;
use crate::utils;
use crate::network::behaviour::Topics;

use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::UnboundedSender;
use libp2p::gossipsub;

pub const DEFAULT_DIFFICULTY_IN_SECONDS: f64 = 10.0;
pub const DEFAULT_NUM_OF_SIDELINKS: usize = 5;

#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkEvent {
    Init(Option<f64>, Option<usize>),
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

#[derive(Serialize, Deserialize, Debug)]
enum NetworkEventInternal {
    Init(Chain),
    BlockProposal(Block),
    RemoteChainRequest { asked_peer_id: String },
    RemoteChainResponse { remote_chain: Chain, chain_receiver: String },
    Message { message: String, from_peer_id: String },
}

impl NetworkEventInternal {
    pub fn _to_string(&self) -> String {
        serde_json::to_string(&self).expect("can serialize network event")
    }

    pub fn from_string(string: &str) -> NetworkEventInternal {
        serde_json::from_str(&string).expect("can deserialize network event")
    }
}

fn send_network_event(
    pending_event: NetworkEventInternal,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
) {
    let topic = match pending_event {
        NetworkEventInternal::Init(_) => Topics::Chain,
        NetworkEventInternal::BlockProposal(_) => Topics::Block,
        NetworkEventInternal::RemoteChainRequest { .. } => Topics::Chain,
        NetworkEventInternal::RemoteChainResponse { .. } => Topics::Chain,
        NetworkEventInternal::Message { .. } => Topics::Message
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
            NetworkEvent::Init(diff, num_sidel) => {
                if std::path::Path::new(blockchain_file).exists() {
                    println!("Blockchain exists. Not initializing the blockchain");
                    return;
                }
                println!("Sending to other peers Init event with difficulty: {:?} and number of sidelinks: {:?}", diff, num_sidel);
                let network_difficulty_secs: f64 = diff.unwrap_or(DEFAULT_DIFFICULTY_IN_SECONDS);
                let hashrate: f64 = utils::find_my_hashrate() as f64;
                println!("My hashrate: {}", hashrate);
                // Difficulty should be such that number of seconds to mine a block is 10
                // Since max hash value for sha256 is 2^256-1, we can calculate the difficulty
                // as 2^256-1 / (10 * hashrate)
                let difficulty = (2.0f64.powi(256) - 1.0) / (network_difficulty_secs * hashrate);
                let num_side_links: usize = num_sidel.unwrap_or(DEFAULT_NUM_OF_SIDELINKS);
                
                let mut blockchain = Chain::new(difficulty, num_side_links);
                blockchain.init_first_block();
                blockchain.save_to_file(blockchain_file);

                println!("Sending Init event with blockchain[difficulty: {difficulty},
                    sidelinks: {num_side_links}]: {:?}", blockchain);

                let pending_event = NetworkEventInternal::Init(blockchain);
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
                let message = NetworkEventInternal::Message {
                    message: message.clone(),
                    from_peer_id: from_peer_id.clone(),
                };
                send_network_event(message, swarm);
            },
        }
    }
}

pub fn handle_incoming_event(event_data: &String,
    local_peer_id: &libp2p::PeerId,
    internal_ch: &UnboundedSender<InternalResponse>,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
    blockchain_file: &str,
) {
    let event = NetworkEventInternal::from_string(event_data);
    match event {
        NetworkEventInternal::Init(Chain) => {
            println!("Received Init event");
            let hashrate = utils::find_my_hashrate();
            // Difficulty should be such that number of seconds to mine a block is 10
            // Since max hash value for sha256 is 2^256-1, we can calculate the difficulty
            // as 2^256-1 / (10 * hashrate)
            // let difficulty = 
            // let blockchain = Chain::new(, num_side_links)
        }
        NetworkEventInternal::BlockProposal(block) => {
            println!("Received BlockProposal event: {:?}", block);
        }
        NetworkEventInternal::RemoteChainRequest { asked_peer_id } => {
            println!("Received RemoteChainRequest event: {:?}", asked_peer_id);
            if asked_peer_id == local_peer_id.to_string() {
                println!("Sending local chain to {}", asked_peer_id);
                let chain = Chain::load_from_file(blockchain_file);
                let event = NetworkEventInternal::RemoteChainResponse {
                    remote_chain: chain,
                    chain_receiver: local_peer_id.to_string(),
                };
                send_network_event(event, swarm);
            }
        }
        NetworkEventInternal::RemoteChainResponse { remote_chain, chain_receiver } => {
            println!("Received RemoteChainResponse event: {:?} from {:?}", remote_chain, chain_receiver);
            if chain_receiver == local_peer_id.to_string() {
                println!("RemoteChainResponse is meant for {}, which is me", chain_receiver);
                // Compare the received chain with the local chain and choose the one with
                // the highest difficulty
                let mut local_chain = Chain::load_from_file(blockchain_file);
                local_chain.choose_longest_chain(&remote_chain);
                local_chain.save_to_file(blockchain_file);
            }
        }
        NetworkEventInternal::Message { message, from_peer_id } => {
            println!("Received Message event: {:?} from {:?}", message, from_peer_id);
        }
    }
}
