use libp2p::{floodsub::Floodsub, futures::channel::mpsc};

pub static KEYS: Lazy = Lazy::new(identity::Keypair::generate_id25519);
pub static PEER_ID: Lazy = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy = Lazy::new(|| Topic::new("blocks"));

pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}

#[derive(NetworkBehavior)]
pub struct AppBehavior {
    // pub sub protocol to communicate.
    pub flood: Floodsub,
    pub mdns: Mdns,
    #[behavior(ignore)]
    pub response_sender: mpsc::UnboundedSender,
    #[behavior(ignore)]
    pub init_sender: mpsc::UnboundedSender,
    #[behavior(ignore)]
    pub app: App,
}
