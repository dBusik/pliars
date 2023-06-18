use serde::{Serialize, Deserialize};
use libp2p::gossipsub;

use crate::blockchain::block;
use crate::blockchain::{
    block::Block,
    chain::{Chain, DIFFICULTY_VALUE},
};
use crate::BlockchainBehaviour;
use crate::utils;
use crate::network::behaviour::Topics;

pub static mut CHAIN_INITIALIZATION_DONE: bool = false;

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    pub fn variant_name(&self) -> String {
        match self {
            NetworkEvent::InitFromUserIo { .. } => "InitFromUserIo".to_string(),
            NetworkEvent::InitUsingChain(_) => "InitUsingChain".to_string(),
            NetworkEvent::BlockProposal(_) => "BlockProposal".to_string(),
            NetworkEvent::RemoteChainRequest { .. } => "RemoteChainRequest".to_string(),
            NetworkEvent::RemoteChainResponse { .. } => "RemoteChainResponse".to_string(),
            NetworkEvent::Message { .. } => "Message".to_string(),
            NetworkEvent::StartMining => "StartMining".to_string(),
        }
    }

    pub fn send(&self,
        swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
    ) {
        println!("Sending event: {:?}", self.variant_name());

        let topic = match self {
            NetworkEvent::InitUsingChain(_) => Topics::Chain,
            NetworkEvent::BlockProposal(_) => Topics::Block,
            NetworkEvent::RemoteChainRequest { .. } => Topics::Chain,
            NetworkEvent::RemoteChainResponse { .. } => Topics::Chain,
            NetworkEvent::Message { .. } => Topics::Message,
            // If mining or user io event is received, do not send it to other peers
            _ => {
                println!("Received local event: {:?}; local events are not meant to be sent\
                    to other peers", self);
                return;
            },
        };
    
        // println!("WIll publish data: {:?}", serde_json::to_vec(&self).expect("can serialize message"));
        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(
            gossipsub::IdentTopic::new(topic.to_string()),
            serde_json::to_vec(&self).expect("can serialize message"))
        {
            if let libp2p::gossipsub::PublishError::InsufficientPeers = e {
                println!("No peers to share event with :(");
            } else {
                panic!("Error while publishing message: {:?}", e);
            }
        } else {
            println!("Event sent successfully: {:?}", self);
        }
    }
}
