use crate::blockchain::{
    chain::{Chain, ChainType, ChainChoice, find_longest_chain, NUM_SIDELINKS},
    block::Block,
};
use crate::BlockchainBehaviour;
use super::event::{NetworkEvent, CHAIN_INITIALIZATION_DONE};
use tokio::sync::mpsc;

#[derive(Debug, PartialEq)]
enum ChainAndFileValidity {
    ValidChainAndFile,
    InvalidChain,
    InvalidFile,
}

fn verify_and_save_chain(chain: &Chain, blockchain_file: &str) -> ChainAndFileValidity {
    print!("Validating the chain and writing it to the file...");
    let chain_valid = chain.validate_chain();
    let chain_saved = if chain_valid {
        chain.save_blockchain_to_file(blockchain_file).is_ok()
    } else {
        false
    };

    match (chain_valid, chain_saved) {
        (true, true) => {
            println!("[SUCCESS] Chain is valid and saved to file");
            ChainAndFileValidity::ValidChainAndFile
        },
        (true, false) => {
            println!("[FAIL] Chain is valid but could not be saved to file");
            ChainAndFileValidity::InvalidFile
        }
        // (false, true) is not possible
        (false, _) => {
            println!("[FAIL] Chain is invalid");
            ChainAndFileValidity::InvalidChain
        }
    }
}

// Function to handle received chain in cases when there is some chain already present
fn choose_chain(remote_chain: Chain,
    blockchain_file: &str
) -> Option<ChainChoice> {
    // Compare the received chain with the local chain and choose the one with
    // the highest difficulty
    let mut winner_chain_choice: Option<ChainChoice> = None;
    if unsafe { CHAIN_INITIALIZATION_DONE } {
        if let Ok(local_chain) = Chain::load_from_file(blockchain_file) {
            winner_chain_choice = Some(find_longest_chain(&local_chain, &remote_chain));
        }
    }

    // If the chain was not initialized or we could not load the local chain from file
    if winner_chain_choice.is_none() {
        println!("Local chain did not load from file successfully.\
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

    if winner_chain_choice.is_some() {
        // If remote chain won store it as local chain\
        let unwrapped_choice = winner_chain_choice.as_ref().unwrap();
        if let ChainType::Remote = unwrapped_choice.chosen_chain_type {
            if let Some(remote_chain) = unwrapped_choice.chosen_chain.as_ref() {
                if remote_chain.save_blockchain_to_file(blockchain_file).is_err() {
                    println!("Error while saving remote blockchain to file,\
                        cancelling the init event");
                    winner_chain_choice = Some(ChainChoice {
                        chosen_chain_type: ChainType::NoChain,
                        chosen_chain: None,
                    });
                }
                println!("Remote chain saved to file")
            }
        }
    }

    winner_chain_choice
}

fn handle_chain_choice_result(chosen_chain: Option<ChainChoice>,
    local_chain_file: &str,
    new_last_block_tx: &mpsc::UnboundedSender<Block>,
    chain_received_from_peer_id: &libp2p::PeerId,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>
) {
    // Only print the type of chain that won if none won or if remote chain won,
    // but if local chain won, propagate to others since it is longer/better
    if chosen_chain.is_none() {
        println!("Received None as choice of the chain result.\
            Should be at least NoChain if a comparison was made. There is a logic error in code.");
        return;
    }
    let chosen_chain = chosen_chain.unwrap();
    match chosen_chain.chosen_chain_type {
        ChainType::NoChain => {
            println!("Both chains were invalid.");
            // TODO: myabe do something about this? Like loading only genesis block into
            // the file
            // let mut blank_chain = unsafe { Chain::new(
            //     DIFFICULTY_VALUE.clone(),
            //     NUM_SIDELINKS,
            // ) };
            // blank_chain.add_block(Block::genesis());
            // blank_chain.save_blockchain_to_file(local_chain_file).unwrap();
            // let event = NetworkEvent::RemoteChainResponse{
            //     chain_from_sender: blank_chain,
            //     chain_receiver: chain_received_from_peer_id.to_string(),
            // };
            // event.send(swarm);
        },
        ChainType::Local => {
            println!("Local chain won.");
            if let Some(local_chain) = chosen_chain.chosen_chain {
                let event = NetworkEvent::RemoteChainResponse{
                    chain_from_sender: local_chain,
                    chain_receiver: chain_received_from_peer_id.to_string(),
                };
                event.send(swarm);
            }
        },
        ChainType::Both => {
            println!("Chains were equal.");
        },
        ChainType::Remote => {
            new_last_block_tx.send(chosen_chain.chosen_chain
                .unwrap()
                .get_last_block()
                .unwrap()
                .clone()
            ).unwrap();
            println!("Remote chain from peer {} won.",
                chain_received_from_peer_id.to_string());
        },
    }
}

fn handle_remote_chain_if_local_uninitialized(remote_chain: Chain,
    local_chain_file: &str,
    new_last_block_tx: &mpsc::UnboundedSender<Block>,
    received_from_peer_id: &libp2p::PeerId,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>
) {
    let remote_chain_save_result = verify_and_save_chain(&remote_chain,
        local_chain_file);
    match remote_chain_save_result {
        ChainAndFileValidity::ValidChainAndFile => {
            // TODO: calculate my hashrate and new difficulty and propagate it to other peers?
            println!("Received remote chain from {} and saved it to file",
                received_from_peer_id.to_string());
            unsafe {
                CHAIN_INITIALIZATION_DONE = true;
            }
            new_last_block_tx.send(remote_chain
                .get_last_block()
                .unwrap()
                .clone()
            ).unwrap();
        },
        ChainAndFileValidity::InvalidChain => {
            // Ask the other peer for the chain again
            println!("Received remote chain from {} but it is invalid. \
                Ignoring it.", received_from_peer_id.to_string());
            // TODO: alternatively look for somebody else with the chain?
            // (But they would have sent the block anyway)
            return;
        },
        ChainAndFileValidity::InvalidFile => {
            // If file was invalid the error is on receiver's side so we
            // don't ask for the chain again but assume the user checks
            // what is going on with the file (obviously if the problem was
            // with, e.g. chain's encoding, the user will just ask for the
            // chain but manually)
            println!("Error while saving remote blockchain to file,\
                cancelling the init event");
            return;
        }
    }
}

pub fn handle_incoming_network_event(event_data: &String,
    local_peer_id: &libp2p::PeerId,
    received_from_peer_id: &libp2p::PeerId,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
    new_last_block_tx: &mpsc::UnboundedSender<Block>,
    local_chain_file: &str,
) {
    let event = NetworkEvent::from_string(event_data);
    println!("Received event: {:?}", event);
    match event {
        NetworkEvent::InitUsingChain(remote_chain) => {
            if unsafe { !CHAIN_INITIALIZATION_DONE } {
                // TODO: calculate my hashrate and new difficulty and propagate it to other peers?
                handle_remote_chain_if_local_uninitialized(remote_chain,
                    local_chain_file,
                    &new_last_block_tx,
                    received_from_peer_id,
                    swarm);
            } else {
                let chosen_chain = choose_chain(
                    remote_chain,
                    local_chain_file);
                
                handle_chain_choice_result(chosen_chain,
                    &local_chain_file,
                    &new_last_block_tx,
                    received_from_peer_id,
                    swarm);
            }
        }
        NetworkEvent::BlockProposal(block) => {
            // Validate the block, if valid add it to the chain and send to the mining task
            // since now it should use this block as the last block in the chain
            if Chain::validate_block_using_file(&block, local_chain_file) {
                println!("Block is valid");
                let block_copy = block.clone();
                if let Err(e) = new_last_block_tx.send(block_copy) {
                    println!("error sending new mined block via channel, {}", e);
                } else {
                    println!("Sent new mined block via channel");
                    if let Err(e) = Chain::append_block_to_file(&block, local_chain_file) {
                        println!("Error while appending block to file: {}", e);
                    }
                }
            } else {
                println!("Block validation failed, asking the peer for the whole chain.");
                let event = NetworkEvent::RemoteChainRequest {
                    asked_peer_id: received_from_peer_id.to_string(),
                };
                event.send(swarm);
            }
        }
        NetworkEvent::RemoteChainRequest { asked_peer_id } => {
            if asked_peer_id == local_peer_id.to_string() {
                println!("Sending local chain to {}", asked_peer_id);
                // Check if chain is ok and ignore if not
                if let Ok(local_chain) = Chain::load_from_file(local_chain_file) {
                    let event = NetworkEvent::RemoteChainResponse {
                        chain_from_sender: local_chain,
                        chain_receiver: received_from_peer_id.to_string(),
                    };
                    event.send(swarm);
            } else {
                    println!("Chain is not valid. Ignoring RemoteChainRequest event from {}",
                        received_from_peer_id.to_string());
                };
            }
        }
        NetworkEvent::RemoteChainResponse { chain_from_sender: remote_chain, chain_receiver } => {
            // Same as InitUsingChain event but check whether the chain was addressed to us
            if chain_receiver == local_peer_id.to_string() {
                if unsafe { !CHAIN_INITIALIZATION_DONE } {
                    handle_remote_chain_if_local_uninitialized(remote_chain,
                        local_chain_file,
                        &new_last_block_tx,
                        received_from_peer_id,
                        swarm);
                } else {
                    println!("Received local chain from {}", received_from_peer_id.to_string());
                    let chosen_chain = choose_chain(
                        remote_chain,
                        local_chain_file);
                    
                    handle_chain_choice_result(chosen_chain,
                        &local_chain_file,
                    &new_last_block_tx,
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
            println!("For some reason received {:?} event from network.\
                Ignoring it.", event);
        }
    }
}
