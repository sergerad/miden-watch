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
    pub account_storage_mode: String,
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
    /// Standard note kind (P2ID/P2IDE/SWAP/PSWAP/MINT/BURN) for public notes, else None.
    pub standard_type: Option<String>,
    /// For public P2ID/P2IDE notes, the addressee account id (hex), else None.
    pub target: Option<String>,
}

/// Live public state of an account, fetched on demand from the node.
#[derive(Debug, Clone)]
pub struct AccountLiveState {
    pub nonce: String,
    pub num_assets: usize,
    pub assets: Vec<String>,
    pub storage_commitment: String,
}

/// A flattened account view: local history plus (optional) live public state.
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub account_id: String,
    /// Account type string (regular/faucet), from AccountId::account_type().
    pub account_type: String,
    /// Whether the account has public on-chain state.
    pub is_public: bool,
    /// Live public state; None for private accounts or when the fetch returned nothing.
    pub live_state: Option<AccountLiveState>,
    /// RPC error text, if the live-state fetch failed (local history is still shown).
    pub error: Option<String>,
    /// Transactions authored by this account (local, from observed blocks).
    pub txs: Vec<TransactionInfo>,
    /// Notes created by this account (sender == id).
    pub sent_notes: Vec<NoteInfo>,
    /// Notes addressed to this account (P2ID/P2IDE target == id).
    pub received_notes: Vec<NoteInfo>,
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
