use color_eyre::eyre::Result;
use miden_client::account::{Account, AccountId};
use miden_client::asset::Asset;
use miden_client::block::BlockNumber;
use miden_client::note::{P2idNoteStorage, StandardNote};
use miden_client::rpc::{Endpoint, GrpcClient, NodeRpcClient};

use crate::types::{AccountLiveState, BlockInfo, NoteInfo, TransactionInfo};

pub struct RpcClient {
    client: GrpcClient,
}

pub struct SyncResult {
    pub block: BlockInfo,
    pub transactions: Vec<TransactionInfo>,
    pub notes: Vec<NoteInfo>,
}

impl RpcClient {
    pub fn new(endpoint: &Endpoint) -> Self {
        let client = GrpcClient::new(endpoint, 10_000);
        Self { client }
    }

    pub async fn get_chain_tip(&self) -> Result<u32> {
        let (header, _) = self.client.get_block_header_by_number(None, false).await?;
        Ok(header.block_num().as_u32())
    }

    pub async fn fetch_block_data(&self, block_num: u32) -> Result<SyncResult> {
        let bn = BlockNumber::from(block_num);

        // Get the block header
        let (header, _) = self
            .client
            .get_block_header_by_number(Some(bn), false)
            .await?;
        let mut block = BlockInfo::from_header(&header);

        // Try to get the full block with transactions and notes
        let mut transactions = Vec::new();
        let mut notes = Vec::new();

        match self.client.get_block_by_number(bn, false).await {
            Ok(proven_block) => {
                let body = proven_block.body();

                // Extract transactions
                for tx_header in body.transactions().as_slice() {
                    transactions.push(TransactionInfo {
                        tx_id: tx_header.id().to_hex(),
                        account_id: tx_header.account_id().to_hex(),
                        account_storage_mode: tx_header
                            .account_id()
                            .account_type()
                            .to_string(),
                        block_num,
                        input_note_count: tx_header.input_notes().iter().count(),
                        output_note_count: tx_header.output_notes().len(),
                    });
                }

                // Extract notes
                for (note_index, output_note) in body.output_notes() {
                    // Classify the standard note kind and, for P2ID/P2IDE, the addressee.
                    // Only public notes carry a recipient (script + storage) in the block body.
                    let (standard_type, target) = match output_note.recipient() {
                        Some(recipient) => {
                            let kind = StandardNote::from_script(recipient.script());
                            let target = match kind {
                                Some(StandardNote::P2ID) | Some(StandardNote::P2IDE) => {
                                    P2idNoteStorage::try_from(recipient.storage().items())
                                        .ok()
                                        .map(|s| s.target().to_hex())
                                }
                                _ => None,
                            };
                            (kind.map(|k| k.name().to_string()), target)
                        }
                        None => (None, None),
                    };

                    notes.push(NoteInfo {
                        note_id: output_note.id().to_hex(),
                        block_num,
                        sender: output_note.metadata().sender().to_hex(),
                        note_type: format!("{:?}", output_note.metadata().note_type()),
                        tag: output_note.metadata().tag().as_u32(),
                        note_index: note_index.leaf_index().position() as u32,
                        standard_type,
                        target,
                    });
                }

                block.tx_count = transactions.len();
                block.note_count = notes.len();
            }
            Err(_) => {
                // If GetBlockByNumber fails, we still have the header
            }
        }

        Ok(SyncResult {
            block,
            transactions,
            notes,
        })
    }

    /// Fetch an account's metadata and (for public accounts) its live on-chain state.
    ///
    /// Returns `(account_type, is_public, live_state, error)`. A malformed hex id is a hard error
    /// (`Err`); a valid id whose RPC fetch fails still returns its metadata with `error` set so the
    /// caller can render local history. `live_state` is `None` for private accounts.
    pub async fn get_account_view(
        &self,
        id_hex: &str,
    ) -> Result<(String, bool, Option<AccountLiveState>, Option<String>)> {
        let id = AccountId::from_hex(id_hex.trim())?;
        let account_type = format!("{}", id.account_type());
        let is_public = id.is_public();
        match self.client.get_account_details(id).await {
            Ok(details) => Ok((account_type, is_public, details.map(flatten_account), None)),
            Err(e) => Ok((account_type, is_public, None, Some(format!("{e}")))),
        }
    }
}

/// Flatten a live [`Account`] into the UI's [`AccountLiveState`].
fn flatten_account(account: Account) -> AccountLiveState {
    let assets: Vec<String> = account
        .vault()
        .assets()
        .map(|asset| match asset {
            Asset::Fungible(fa) => format!("{} @ {}", fa.amount(), fa.faucet_id().to_hex()),
            Asset::NonFungible(nfa) => format!("NFT @ {}", nfa.faucet_id().to_hex()),
        })
        .collect();
    AccountLiveState {
        nonce: account.nonce().to_string(),
        num_assets: assets.len(),
        storage_commitment: account.storage().to_commitment().to_hex(),
        assets,
    }
}
