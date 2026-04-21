use color_eyre::eyre::Result;
use miden_client::block::BlockNumber;
use miden_client::rpc::{Endpoint, GrpcClient, NodeRpcClient};

use crate::types::{BlockInfo, NoteInfo, TransactionInfo};

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

    pub async fn get_block_header(&self, block_num: Option<u32>) -> Result<BlockInfo> {
        let bn = block_num.map(BlockNumber::from);
        let (header, _) = self.client.get_block_header_by_number(bn, false).await?;
        Ok(BlockInfo::from_header(&header))
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

        match self.client.get_block_by_number(bn).await {
            Ok(proven_block) => {
                let body = proven_block.body();

                // Extract transactions
                for tx_header in body.transactions().as_slice() {
                    transactions.push(TransactionInfo {
                        tx_id: tx_header.id().to_hex(),
                        account_id: tx_header.account_id().to_hex(),
                        account_storage_mode: tx_header
                            .account_id()
                            .storage_mode()
                            .to_string(),
                        block_num,
                        input_note_count: tx_header.input_notes().iter().count(),
                        output_note_count: tx_header.output_notes().len(),
                    });
                }

                // Extract notes
                for (note_index, output_note) in body.output_notes() {
                    notes.push(NoteInfo {
                        note_id: output_note.id().to_hex(),
                        block_num,
                        sender: output_note.metadata().sender().to_hex(),
                        note_type: format!("{:?}", output_note.metadata().note_type()),
                        tag: output_note.metadata().tag().as_u32(),
                        note_index: note_index.leaf_index().position() as u32,
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
}
