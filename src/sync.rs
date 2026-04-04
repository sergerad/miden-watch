use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result;
use miden_client::rpc::Endpoint;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::rpc::RpcClient;
use crate::store::Store;
use crate::types::StartFrom;

pub async fn run_sync(
    endpoint: Endpoint,
    start_from: StartFrom,
    store: Arc<Mutex<Store>>,
    action_tx: mpsc::UnboundedSender<Action>,
) -> Result<()> {
    let rpc = RpcClient::new(&endpoint);

    // Determine starting block
    let explicit_start = !matches!(start_from, StartFrom::Tip);
    let mut current_block = match start_from {
        StartFrom::Genesis => 0,
        StartFrom::Block(n) => n,
        StartFrom::Tip => match rpc.get_chain_tip().await {
            Ok(tip) => tip,
            Err(e) => {
                let _ = action_tx.send(Action::SyncError(format!("Failed to get chain tip: {e}")));
                0
            }
        },
    };

    // Check if we already have data in the store
    {
        let store = store.lock().await;
        if let Ok(Some(latest)) = store.get_latest_block_num() {
            // Load cached blocks into UI
            if let Ok(blocks) = store.get_blocks(100, 0) {
                for block in blocks.into_iter().rev() {
                    let block_num = block.block_num;
                    let txs = store
                        .get_transactions_for_block(block_num)
                        .unwrap_or_default();
                    let notes = store.get_notes_for_block(block_num).unwrap_or_default();
                    let _ = action_tx.send(Action::NewBlockReceived {
                        block,
                        transactions: txs,
                        notes,
                    });
                }
            }
            // If start wasn't explicitly set, continue from where we left off
            if !explicit_start && latest >= current_block {
                current_block = latest + 1;
            }
        }
    }

    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        // Get the chain tip
        let chain_tip = match rpc.get_chain_tip().await {
            Ok(tip) => tip,
            Err(e) => {
                let _ = action_tx.send(Action::SyncError(format!("Failed to get chain tip: {e}")));
                continue;
            }
        };

        // Fetch all blocks from current_block to chain_tip
        while current_block <= chain_tip {
            match rpc.fetch_block_data(current_block).await {
                Ok(sync_result) => {
                    let store = store.lock().await;
                    let _ = store.insert_block(&sync_result.block);
                    let _ = store.insert_transactions(&sync_result.transactions);
                    let _ = store.insert_notes(&sync_result.notes);
                    drop(store);

                    let _ = action_tx.send(Action::NewBlockReceived {
                        block: sync_result.block,
                        transactions: sync_result.transactions,
                        notes: sync_result.notes,
                    });
                }
                Err(e) => {
                    let _ = action_tx.send(Action::SyncError(format!(
                        "Failed to fetch block {}: {e}",
                        current_block
                    )));
                    break;
                }
            }
            current_block += 1;
        }
    }
}
