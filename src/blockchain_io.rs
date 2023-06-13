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
        help                                - print this message
        listpeers                           - print peers
        init <difficulty> <num sidelinks>   - initialize the blockchain
        listblocks [file]                   - print blocks (whole blockchain), optionally into a file
        addrecord <data>                    - add record to the last block of the chain
        printblock  <block index>           - display contents of a chosen block
        numberblocks                        - display number of blocks in the chain
        talk <message>                      - send a text message to all other peers (will wave if no message is provided)
        exit                                - exit the program
 */

pub fn print_cmd_options() {
    println!("Possible commands:\n\
        \thelp                              - print this message\n\
        \tinit <difficulty> <num sidelinks> - initialize the blockchain\n\
        \tlistpeers                         - print peers\n\
        \tlistblocks [file]                 - *pretty* print blocks (whole blockchain), optionally into a file\n\
        \taddrecord <data>                  - add record to the last block of the chain\n\
        \tprintblock  <block index>         - display contents of a chosen block\n\
        \tnumberblocks                      - display number of blocks in the chain\n\
        \ttalk <message>                    - send a text message to all other peers (will wave if no message is provided)\n\
        \texit                              - exit the program"
    );
}

pub fn process_cmd(user_input: String,
    swarm: &mut libp2p::Swarm<BlockchainBehaviour>,
    local_peer_id: &libp2p::PeerId,
    blockchain_file: &str,
) {
    let mut user_input = user_input.split_whitespace();
    match user_input.next() {
        Some("help") => {
            print_cmd_options();
        },
        Some("init") => {
            println!("init received");
            let network_difficulty_secs: Option<f64> = if let Some(difficulty) = user_input.next() {
                let diff_val = difficulty.parse().expect("can parse difficulty");
                Some(diff_val)
            } else {
                None
            };

            let num_side_links: Option<usize> = if let Some(sidelinks_num) = user_input.next() {
                let sidel_val = sidelinks_num.parse().expect("can parse number of sidelinks");
                Some(sidel_val)
            } else {
                None
            };

            let event = NetworkEvent::Init(network_difficulty_secs, num_side_links);
            event.send(swarm, blockchain_file);
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
        Some("listblocks") => {
            println!("listblocks received");
            if let Ok(local_chain) = Chain::load_from_file(blockchain_file) {
                let blockchain = serde_json::to_string_pretty(&local_chain).expect("can serialize blockchain");
                match user_input.next() {
                    Some(file_name) => {
                        println!("Writing blockchain to file {}", file_name);
                        let mut file = File::create(file_name).expect("can create file");
                        file.write_all(blockchain.as_bytes()).expect("can write to file");
                    },
                    None => {
                        println!("{}", blockchain);
                    }
                }
            } else {
                println!("Cannot load blockchain from file");
            }
        },
        // Some("addrecord") => {
        //     println!("addrecord received");
        //     let mut blockchain = blockchain::chain::Chain::new();
        //     blockchain.load_from_file("blockchain.json");
        //     let mut blockchain = blockchain.get_chain();
        //     let mut block = blockchain.pop().expect("can pop block");
        //     let mut data = String::new();
        //     for word in user_input {
        //         data.push_str(word);
        //         data.push(' ');
        //     }
        //     data.pop();
        //     block.add_record(data);
        //     blockchain.push(block);
        //     let blockchain = blockchain::chain::Chain::from_vec(blockchain);
        //     blockchain.save_to_file("blockchain.json");
        // },
        Some("printblock") => {
            println!("printblock received");
            let block_index = user_input.next().expect("can get block index").parse::<usize>().expect("can parse block index");
            let block = Block::load_block_from_file(
                block_index,
                blockchain_file)
            .expect("can load blockchain");
            println!("{:#?}", block);
        },
        Some("numberblocks") => {
            println!("numberblocks received");
            let blockchain_length = Chain::get_blockchain_length(blockchain_file);
            println!("Number of blocks: {}", blockchain_length);
        },
        Some("talk") => {
            println!("talk received");
            let fallback_msg = format!("Hello from {}", local_peer_id.to_string());
            let message = user_input.next().unwrap_or(fallback_msg.as_str());
            let event = NetworkEvent::Message {
                message: message.to_string(),
                from_peer_id: local_peer_id.to_string(),
            };
            event.send(swarm, blockchain_file);
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