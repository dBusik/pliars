use crate::blockchain::{
    chain::{Chain, ChainType, ChainChoice, find_longest_chain},
};
use crate::BlockchainBehaviour;
use super::CHAIN_INITIALIZED;
use super::event::{NetworkEvent, send_network_event};

#[derive(Debug, PartialEq)]
enum ChainAndFileValidity {
    ValidChainAndFile,
    InvalidChain,
    InvalidFile,
}

fn verify_and_save_chain(chain: &Chain, blockchain_file: &str) -> ChainAndFileValidity {
    let chain_valid = chain.validate_chain();
    let chain_saved = if chain_valid {
        chain.save_blockchain_to_file(blockchain_file).is_ok()
    } else {
        false
    };

    match (chain_valid, chain_saved) {
        (true, true) => ChainAndFileValidity::ValidChainAndFile,
        (true, false) => ChainAndFileValidity::InvalidFile,
        // (false, true) is not possible
        (false, _) => ChainAndFileValidity::InvalidChain,
    }
}

// Function to handle received chain in cases when there is some chain already present
fn choose_chain(remote_chain: Chain,
    blockchain_file: &str
) -> Option<ChainChoice> {
    // Compare the received chain with the local chain and choose the one with
    // the highest difficulty
    let mut winner_chain_choice: Option<ChainChoice> = None;
    if unsafe { CHAIN_INITIALIZED } {
        if let Ok(local_chain) = Chain::load_from_file(blockchain_file) {
            winner_chain_choice = Some(find_longest_chain(&local_chain, &remote_chain));
        }
    }

    // If the chain was not initialized or we could not load the local chain from file
    if winner_chain_choice.is_none() {
        println!("Local chain did not load from file successfully.
        Veryfiyng remote chain and saving it as local chain");
        let remote_chain_valid_and_saved = verify_and_save_chain(&remote_chain,
            blockchain_file);

        if remote_chain_valid_and_saved == ChainAndFileValidity::ValidChainAndFile {
            winner_chain_choice = Some(ChainChoice {
                chosen_chain_type: ChainType::Remote,
                chosen_chain: Some(remote_chain),
            });
        } else {
            winner_chain_choice = Some(ChainChoice {
                chosen_chain_type: ChainType::NoChain,
                chosen_chain: None,
            });
        }
    };

    winner_chain_choice
}

fn handle_chain_choice_result(chosen_chain: Option<ChainChoice>,
    chain_received_from_peer_id: &libp2p::PeerId,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>
) {
    // Only print the type of chain that won if none won or if remote chain won,
    // but if local chain won, propagate to others since it is longer/better
    if chosen_chain.is_none() {
        println!("Received None as choice of the chain result.
            Should be at least NoChain if a comparison was made. There is a logic error in code.");
        return;
    }
    let chosen_chain = chosen_chain.unwrap();
    match chosen_chain.chosen_chain_type {
        ChainType::NoChain => {
            println!("No chain won.");
        },
        ChainType::Local => {
            println!("Local chain won.");
            if let Some(local_chain) = chosen_chain.chosen_chain {
                let event = NetworkEvent::RemoteChainResponse{
                    chain_from_sender: local_chain,
                    chain_receiver: chain_received_from_peer_id.to_string(),
                };
                send_network_event(event, swarm);
            }
        },
        ChainType::Both => {
            println!("Chains were equal.");
        },
        ChainType::Remote => {
            println!("Remote chain won.");
        },
    }
}

fn handle_remote_chain_if_local_uninitialized(remote_chain: Chain,
    local_chain_file: &str,
    received_from_peer_id: &libp2p::PeerId,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>
) {
    let remote_chain_save_result = verify_and_save_chain(&remote_chain,
        local_chain_file);
    match remote_chain_save_result {
        ChainAndFileValidity::ValidChainAndFile => {
            // TODO: calculate my hasrate and new difficulty and propagate it to other peers?
            println!("Received remote chain from {} and saved it to file",
                received_from_peer_id.to_string());
            unsafe { CHAIN_INITIALIZED = true; }
        },
        ChainAndFileValidity::InvalidChain => {
            // Ask the other peer for the chain again
            println!("Received remote chain from {} but it is invalid.
                Ignoring it", received_from_peer_id.to_string());
            let event = NetworkEvent::RemoteChainRequest {
                asked_peer_id: received_from_peer_id.to_string(),
            };
            send_network_event(event, swarm);
            return;
        },
        ChainAndFileValidity::InvalidFile => {
            // If file was invalid the error is on receiver's side so we
            // don't ask for the chain again but assume the user checks
            // what is going on with the file (obviously if the problem was
            // with, e.g. chain's encoding, the user will just ask for the
            // chain but manually)
            println!("Error while saving remote blockchain to file,
                cancelling the init event");
            return;
        }
    }
}

pub fn handle_incoming_network_event(event_data: &String,
    local_peer_id: &libp2p::PeerId,
    received_from_peer_id: &libp2p::PeerId,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
    local_chain_file: &str,
) {
    let event = NetworkEvent::from_string(event_data);
    match event {
        NetworkEvent::InitUsingChain(remote_chain) => {
            if unsafe { !CHAIN_INITIALIZED } {
                // TODO: calculate my hashrate and new difficulty and propagate it to other peers?
                handle_remote_chain_if_local_uninitialized(remote_chain,
                    local_chain_file,
                    received_from_peer_id,
                    swarm);
            } else {
                let chosen_chain = choose_chain(
                    remote_chain,
                    local_chain_file);
                
                handle_chain_choice_result(chosen_chain, received_from_peer_id, swarm);
            }
        }
        NetworkEvent::BlockProposal(block) => {
            println!("Received BlockProposal event: {:?}", block);
        }
        NetworkEvent::RemoteChainRequest { asked_peer_id } => {
            println!("Received RemoteChainRequest event: {:?}", asked_peer_id);
            if asked_peer_id == local_peer_id.to_string() {
                println!("Sending local chain to {}", asked_peer_id);
                // Check if chain is ok and ignore if not
                if let Ok(local_chain) = Chain::load_from_file(local_chain_file) {
                    let event = NetworkEvent::RemoteChainResponse {
                        chain_from_sender: local_chain,
                        chain_receiver: local_peer_id.to_string(),
                    };
                    send_network_event(event, swarm);
                } else {
                    println!("Chain is not valid. Ignoring RemoteChainRequest event from {}",
                        received_from_peer_id.to_string());
                };
            }
        }
        NetworkEvent::RemoteChainResponse { chain_from_sender: remote_chain, chain_receiver } => {
            // Same as InitUsingChain event but check whether the chain was addressed to us
            if chain_receiver == local_peer_id.to_string() {
                if unsafe { !CHAIN_INITIALIZED } {
                    handle_remote_chain_if_local_uninitialized(remote_chain,
                        local_chain_file,
                        received_from_peer_id,
                        swarm);
                } else {
                    println!("Received local chain from {}", received_from_peer_id.to_string());
                    let chosen_chain = choose_chain(
                        remote_chain,
                        local_chain_file);
                    
                    handle_chain_choice_result(chosen_chain,
                        received_from_peer_id,
                        swarm);
                }
            }
        }
        NetworkEvent::Message { message, from_peer_id } => {
            println!("Received Message event: {:?} from {:?}", message, from_peer_id);
        }
        _ => {
            // This events won't actually be sent by other peers, code is present for
            // possible extension of the communication between the peers
            println!("For some reason received {:?} event from network.
                Ignoring it.", event);
        }
    }
}
