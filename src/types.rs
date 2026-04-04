use miden_client::block::BlockHeader;

/// Where to start syncing from.
#[derive(Debug, Clone)]
pub enum StartFrom {
    /// Start from the current chain tip
    Tip,
    /// Start from the genesis block (block 0)
    Genesis,
    /// Start from a specific block number
    Block(u32),
}

impl std::str::FromStr for StartFrom {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tip" | "latest" => Ok(StartFrom::Tip),
            "genesis" => Ok(StartFrom::Genesis),
            other => other.parse::<u32>().map(StartFrom::Block).map_err(|_| {
                format!("expected 'tip', 'genesis', or a block number, got '{other}'")
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub block_num: u32,
    pub timestamp: u32,
    pub version: u32,
    pub prev_block_commitment: String,
    pub chain_commitment: String,
    pub account_root: String,
    pub nullifier_root: String,
    pub note_root: String,
    pub tx_commitment: String,
    pub tx_kernel_commitment: String,
    pub tx_count: usize,
    pub note_count: usize,
}

#[derive(Debug, Clone)]
pub struct TransactionInfo {
    pub tx_id: String,
    pub account_id: String,
    pub block_num: u32,
    pub input_note_count: usize,
    pub output_note_count: usize,
}

#[derive(Debug, Clone)]
pub struct NoteInfo {
    pub note_id: String,
    pub block_num: u32,
    pub sender: String,
    pub note_type: String,
    pub tag: u32,
    pub note_index: u32,
}

impl BlockInfo {
    pub fn from_header(header: &BlockHeader) -> Self {
        Self {
            block_num: header.block_num().as_u32(),
            timestamp: header.timestamp(),
            version: header.version(),
            prev_block_commitment: header.prev_block_commitment().to_hex(),
            chain_commitment: header.chain_commitment().to_hex(),
            account_root: header.account_root().to_hex(),
            nullifier_root: header.nullifier_root().to_hex(),
            note_root: header.note_root().to_hex(),
            tx_commitment: header.tx_commitment().to_hex(),
            tx_kernel_commitment: header.tx_kernel_commitment().to_hex(),
            tx_count: 0,
            note_count: 0,
        }
    }
}
