use std::io::stderr;

use chrono::Utc;
use log::warn;
use serde::{Deserialize, Serialize};

const DIFF_PREFIX: &str = "00";

pub struct App {
    pub blocks: Vec<Block>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: i64,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64,
}

fn hash_to_bin(hash: &[u8]) -> String {
    let mut res: String = String::default();
    for c in hash {
        res.push_str(&format!("{:b}", c));
    }
    res
}

// validation
impl App {
    fn new() -> Self {
        Self { blocks: vec![] }
    }

    // set_genesis - inits the genesis block.
    fn set_genesis(&mut self) {
        let genensis_block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            prev_hash: String::from("genesis"),
            data: String::from("genesis!"),
            nonce: 2836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.blocks.push(genensis_block);
    }

    // is_block_valid - checks to see if the current block is valid to pass through.
    fn is_block_valid(&self, block: &Block, prev: &Block) -> bool {
        if block.prev_hash != prev.hash {
            warn!("block with id: {} has wrong previous hash", block.id);
            return false;
        } else if !hash_to_bin(&hex::decode(&block.hash).expect("can decode from hex"))
            .starts_with(DIFF_PREFIX)
        {
            warn!("block with id {} has invalid difficulty", block.id);
            return false;
        } else if block.id != prev.id + 1 {
            warn!(
                "block with id {} is not the next block after latest: {}",
                block.id, prev.id
            );
            return false;
        } else if hex::encode(calc_hash(block.id, block.timestamp. &block.prev_hash, &block.data, block.nonce)) != block.hash {
            warn!("block with the id {} has invalid hash", block.id);
        }
        return true;
    }
    // try_add_block - tries to add the block to the blockchain.
    fn try_add_block(&mut self, block: Block) {
        let latest_block = self.blocks.last().expect("there is at least one block.");
        // if the latest block is good to go, push to the block.
        if self.is_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            eprintln!("could not add block - invalid op.");
        }
    }
}
fn main() {
    println!("Hello, world!");
}
