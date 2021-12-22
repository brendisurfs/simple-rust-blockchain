pub mod p2p;

use chrono::Utc;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DIFF_PREFIX: &str = "00";

pub struct App {
    pub blocks: Vec<Block>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64,
}

// _____________UTILITIES_______________________________________________________
fn hash_to_bin(hash: &[u8]) -> String {
    let mut res: String = String::default();
    for c in hash {
        res.push_str(&format!("{:b}", c));
    }
    res
}
// calc_hash - calculates the next hash in the lineup
fn calc_hash(id: u64, timestamp: i64, prev_hash: &str, data: &str, nonce: u64) -> Vec<u8> {
    let data = serde_json::json!({
        "id": id,
        "previous_hash": prev_hash,
        "data": data,
        "timestamp": timestamp,
        "nonce": nonce,
    });
    let mut hasher = Sha256::new();
    hasher.update(data.to_string().as_bytes());
    return hasher.finalize().as_slice().to_owned();
}
fn mine_block(id: u64, timestamp: i64, prev_hash: &str, data: &str) -> (u64, String) {
    info!("mining block...");
    let mut nonce = 0;

    // loop until mined.
    loop {
        if nonce % 100000 == 0 {
            info!("nonce: {}", nonce);
        }

        let hash = calc_hash(id, timestamp, prev_hash, data, nonce);
        let bin_hash = hash_to_bin(&hash);
        if bin_hash.starts_with(DIFF_PREFIX) {
            info!(
                "mined a block! nonce: {}\n hash: {} \n bin hash: {}\n",
                nonce,
                hex::encode(&hash),
                bin_hash,
            );
            // now we pass the hash data over, rather than borrowing it.
            return (nonce, hex::encode(hash));
        }
        nonce += 1;
    }
}

// _____________________________________________APP____________________________________
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
        } else if hex::encode(calc_hash(
            block.id,
            block.timestamp,
            &block.prev_hash,
            &block.data,
            block.nonce,
        )) != block.hash
        {
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

    // is_chain_valid - checks if our chain is valid. if not, fail the whole thing.
    fn is_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.is_block_valid(second, first) {
                return false;
            }
        }
        true
    }
    // choose_chain - chooses the longest chain when there is a mining conflict.
    fn choose_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block> {
        // check both the remote and local chains to see whats good.
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);

        // check the validity against each chain.
        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if is_remote_valid && !is_local_valid {
            remote
        } else if !is_remote_valid && is_local_valid {
            local
        } else {
            panic!("local and remote chains are both invalid.");
        }
    }
}

// ___________________________________BLOCK______________________________________________
impl Block {
    pub fn new(id: u64, prev_hash: String, data: String) -> Self {
        let now = Utc::now();
        let (nonce, hash) = mine_block(id, now.timestamp(), &prev_hash, &data);
        Self {
            id,
            hash,
            timestamp: now.timestamp(),
            prev_hash,
            data,
            nonce,
        }
    }
}
fn main() {
    println!("Hello, world!");
}
