use std::collections::HashSet;

use super::{App, Block};
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm},
    NetworkBehaviour, PeerId,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// keypair and peer ID.
// helps identy a client on the p2p network.
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

#[derive(Debug, Deserialize, Serialize)]
// holds llist of blocks ,and a receiver.
// this is the struct we expect when someone sends ups their local chain and use to send out our chain.
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub receiver: String,
}

#[derive(Debug, Deserialize, Serialize)]
/*
triggers the interaction between nodes sharing info.
if we send localchainrequest with a peer_id of another node in the system
it will trigger that they send us  their chain back.
*/
pub struct LocalChainRequest {
    pub from_peer_id: String,
}
// handles incoming messages, input, and initialization.
pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}

// Network Behavior:
// allows for pub/sub communication
// mdns enables auto finding of other nodes.
#[derive(NetworkBehaviour)]
//
pub struct AppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    // macros not working?
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub app: App,
}
// App Behavior handler and function.
impl AppBehaviour {
    pub async fn new(
        app: App,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns conn."),
            response_sender,
            init_sender,
        };
        // what does clone do?
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        // clone returns a copy of value.
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());

        return behaviour;
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        // ruleset: if a new node is discovered, we add it to our list to communicate.
        // when it expires, we remove it from the list.
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    // if has_node is false
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}
// Handle network messages from other nodes.
impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        //
        // pass down the floodsub msg we work with.
        if let FloodsubEvent::Message(msg) = event {
            //
            // Ok (result ) type pulled from Chain Response serde_json.
            if let Ok(response) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                //
                if response.receiver == PEER_ID.to_string() {
                    info!("response from {}: ", msg.source);
                    response.blocks.iter().for_each(|i| info!("{:?}", i));

                    self.app.blocks = self
                        .app
                        .choose_chain(self.app.blocks.clone(), response.blocks)
                }
                // if result Ok from LocalChainRequest type
            } else if let Ok(response) = serde_json::from_slice::<LocalChainRequest>(&msg.data) {
                // we add types to the serde function to enable typing down the line.
                info!("Sending local chain to id: {}", msg.source.to_string());
                let peer_id = response.from_peer_id;
                if PEER_ID.to_string() == peer_id {
                    if let Err(e) = self.response_sender.send(ChainResponse {
                        blocks: self.app.blocks.clone(),
                        receiver: msg.source.to_string(),
                    }) {
                        error!("error sending request vial channel, {}", e);
                    }
                }
            } else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
                info!("received new block from {}", msg.source.to_string());
                self.app.try_add_block(block);
            }
        }
    }
}

// get_list_peers - gets the list of all discovered peers in the network.
pub fn get_list_peers(swarm: &Swarm<AppBehaviour>) -> Vec<String> {
    info!("Discovered network peers: ");

    //
    let nodes = swarm.behaviour().mdns.discovered_nodes();

    //
    let mut unique_peers = HashSet::new();

    for &peer in nodes {
        unique_peers.insert(peer);
    }
    // creates a closure and modifies the data in place.
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn print_peers(swarm: &Swarm<AppBehaviour>) {
    // takes in swarm and prints our our list.
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p))
}

pub fn print_chain(swarm: &Swarm<AppBehaviour>) {
    info!("Local Blockcahin:");
    let pretty_json = serde_json::to_string_pretty(&swarm.behaviour().app.blocks)
        .expect("can parse blocks to json.");
    info!("{}", pretty_json);
}

pub fn create_block(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        let latest_block = behaviour.app.blocks.last().expect("at least one block");
        // construct new block
        let block = Block::new(
            latest_block.id + 1,
            latest_block.hash.clone(),
            data.to_owned(),
        );
        let json_data = serde_json::to_string(&block).expect("can parse request to json");
        behaviour.app.blocks.push(block);
        info!("broadcasting new block to network");
        // finally, publish the block and data to the network.
        behaviour
            .floodsub
            .publish(BLOCK_TOPIC.clone(), json_data.as_bytes());
    }
}
