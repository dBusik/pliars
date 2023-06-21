use crate::blockchain::block;
use crate::network::event::{
    NetworkEvent
};
use crate::network::behaviour::BlockchainBehaviour;

use std::fs::File;
use std::io::Write;
use crate::blockchain::{
    chain::Chain,
    block::Block
};

// TODO: remove all .expect and perform proper error handling

/*
    Possible commands:
        help                                    - print this message
        listpeers                               - print peers
        init d=<difficulty> sl=<num sidelinks>  - initialize the blockchain
        blocks [<start>..<end>|[comma-separated indexes]|n|"all"] [file to write to]
        rec <data>                              - add record to the last block of the chain
        printblock  <block index>               - display contents of a chosen block
        numberblocks                            - display number of blocks in the chain
        talk <message>                          - send a text message to all other peers (will wave if no message is provided)
        myid                                    - print your peer id
        myfile                                  - print your blockchain file path
        exit                                    - exit the program
 */

pub fn print_cmd_options() {
    println!("Possible commands:\n\
        \thelp                                      - print this message\n\
        \tinit d=<difficulty> sl=<num sidelinks>    - initialize the blockchain\n\
        \tlistpeers                                 - print peers\n\
        \tblocks [<start>..<end>|[comma-separated indexes]|n|\"all\"] [file to write to]\n\
        \trec <data>                                - add record to the last block of the chain\n\
        \tprintblock  <block index>                 - display contents of a chosen block\n\
        \tnumberblocks                              - display number of blocks in the chain\n\
        \ttalk <message>                            - send a text message to all other peers (will wave if no message is provided)\n\
        \tmyid                                      - print your peer id\n\
        \tmyfile                                    - print your blockchain file path\n\
        \texit                                      - exit the program"
    );
}

// Processing of the user input which does not involve sending new events to other threads or peers
pub fn process_simple_cmd(user_input: String,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
    local_peer_id: &libp2p::PeerId,
    blockchain_file: &str,
) {
    let mut user_input = user_input.split_whitespace();
    match user_input.next() {
        Some("help") => {
            print_cmd_options();
        },
        Some("listpeers") => {
            println!("listpeers received");
            let peers = swarm.behaviour().gossipsub.all_peers();
            // List all the peers we are connected to
            println!("Connected peers:");
            for peer in peers {
                println!("{:?}", peer);
            }
        },
        Some("blocks") => {
            println!("blocks received");
            let mut blocks_to_read = Vec::new();
            let mut file_to_write_to = None;
            let mut all_blocks = false;
            if let Some(val) = user_input.next() {
                if val == "all" {
                    all_blocks = true;
                } else if val.contains("..") {
                    let mut range = val.split("..");
                    let start = if let Some(val) = range.next() {
                        if let Ok(num) = val.parse::<u64>() {
                            num
                        } else {
                            println!("Cannot parse start index");
                            return;
                        }
                    } else {
                        println!("No start index provided");
                        return;
                    };
                    let end = if let Some(val) = range.next() {
                        if let Ok(num) = val.parse::<u64>() {
                            num
                        } else {
                            println!("Cannot parse end index");
                            return;
                        }
                    } else {
                        println!("No end index provided");
                        return;
                    };
                    if start > end {
                        println!("Start index cannot be greater than end index");
                        return;
                    }
                    blocks_to_read = (start..=end).collect();
                } else if val.contains(",") {
                    blocks_to_read = val.split(",")
                        .map(|x| x.parse::<u64>().unwrap())
                        .collect();
                } else {
                    if let Ok(num) = val.parse::<usize>() {
                        // Read last num blocks
                        let blockchain_length = if let Ok(len) = Chain::get_blockchain_length(blockchain_file) {
                            len
                        } else {
                            println!("Cannot get blockchain length");
                            return;
                        };
                        let num_to_read = if num > blockchain_length {
                            blockchain_length
                        } else {
                            num
                        };
                        blocks_to_read = ((blockchain_length - num_to_read + 1) as u64..=blockchain_length as u64).collect();
                    } else {
                        println!("Cannot parse block index");
                        return;
                    }
                }
            } else {
                println!("No block index provided");
                return;
            }
            if let Some(val) = user_input.next() {
                file_to_write_to = Some(val);
            }
            let blocks = if all_blocks {
                Chain::get_last_n_blocks_from_file(
                    Chain::get_blockchain_length(blockchain_file).unwrap(),
                    blockchain_file)
            } else {
                Chain::get_blocks_by_indices_from_file(
                    blocks_to_read,
                    blockchain_file)
            };
            if let Some(file_to_write_to) = file_to_write_to {
                let mut file = File::create(file_to_write_to).unwrap();
                if let Some(blocks) = blocks {
                    for block in blocks {
                        // file.write_all(format!("{:#?}\n", block).as_bytes()).unwrap();
                        file.write_all(format!("{:?}\n", block).as_bytes()).unwrap();
                    }
                } else {
                    println!("Cannot get blocks from file");
                }
            } else {
                if let Some(blocks) = blocks {
                    for block in blocks {
                        // println!("{:#?}", block);
                        println!("{:?}", block);
                    }
                } else {
                    println!("Cannot get blocks from file");
                }
            }
        },
        Some("printblock") => {
            println!("printblock received");
            let block_index = if let Some(val) = user_input.next() {
                if let Ok(num) = val.parse::<usize>() {
                    num
                } else {
                    println!("Cannot parse block index");
                    return;
                }
            } else {
                println!("No block index provided");
                return;
            };
            let block = if let Some(block) = Chain::load_block_from_file(
                block_index as u64,
                blockchain_file)
            {
                block
            } else {
                println!("Cannot load block from file");
                return;
            };
            println!("{:#?}", block);
        },
        Some("numberblocks") => {
            println!("numberblocks received");
            if let Ok(len) = Chain::get_blockchain_length(blockchain_file) {
                println!("Number of blocks: {}", len);
            } else {
                println!("Cannot get lengt of blockchain from file");
            };
        },
        Some("talk") => {
            println!("talk received");
            let fallback_msg = format!("Hello from {}", local_peer_id.to_string());
            let message = user_input.next().unwrap_or(fallback_msg.as_str());
            let event = NetworkEvent::Message {
                message: message.to_string(),
                from_peer_id: local_peer_id.to_string(),
            };
            event.send(swarm);
        },
        Some("myid") => {
            println!("myid received");
            println!("Your peer id: {}", local_peer_id.to_string());
        },
        Some("myfile") => {
            println!("myfile received");
            println!("Your blockchain file path: {}", blockchain_file);
        },
        Some("exit") => {
            println!("exit received");
            std::process::exit(0);
        },
        Some(cmd) => {
            println!("Unknown command: {}", cmd);
            print_cmd_options();
        },
        None => {
            println!("No command provided");
            print_cmd_options();
        }
    }
}