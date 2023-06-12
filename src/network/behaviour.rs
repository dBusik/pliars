use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, mdns};

#[derive(Clone, Debug)]
pub enum Topics {
    Block,
    Chain,
    Hashrate,
    Message
}

impl ToString for Topics {
    fn to_string(&self) -> String {
        return format!("{:?}", self)
    }
}

#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
}
