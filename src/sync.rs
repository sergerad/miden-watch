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
    stop_block: Option<u32>,
    store: Arc<Mutex<Store>>,
    action_tx: mpsc::UnboundedSender<Action>,
) -> Result<()> {
    let rpc = RpcClient::new(&endpoint);

    // Determine starting block
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

    // Load cached blocks from the store into the UI
    {
        let store = store.lock().await;
        if let Ok(blocks) = store.get_blocks_from(current_block, stop_block) {
            let total = blocks.len() as u32;
            // blocks are ordered newest-first, reverse to process oldest-first
            for (i, block) in blocks.into_iter().rev().enumerate() {
                let block_num = block.block_num;
                let _ = action_tx.send(Action::LoadProgress {
                    current: i as u32 + 1,
                    target: total,
                });
                let txs = store
                    .get_transactions_for_block(block_num)
                    .unwrap_or_default();
                let notes = store.get_notes_for_block(block_num).unwrap_or_default();
                let _ = action_tx.send(Action::NewBlockReceived {
                    block,
                    transactions: txs,
                    notes,
                });
                // Only advance current_block for contiguous coverage
                if block_num == current_block {
                    current_block = block_num + 1;
                }
            }
        }
    }

    let mut interval = tokio::time::interval(Duration::from_secs(2));

    loop {
        interval.tick().await;

        // Get the chain tip and measure latency
        let start = std::time::Instant::now();
        let chain_tip = match rpc.get_chain_tip().await {
            Ok(tip) => {
                let latency = start.elapsed().as_millis() as u64;
                let _ = action_tx.send(Action::Latency(latency));
                tip
            }
            Err(e) => {
                let _ = action_tx.send(Action::SyncError(format!("Failed to get chain tip: {e}")));
                continue;
            }
        };

        // Determine the upper bound for this sync pass
        let sync_until = match stop_block {
            Some(stop) => chain_tip.min(stop),
            None => chain_tip,
        };

        if current_block <= sync_until {
            while current_block <= sync_until {
                let _ = action_tx.send(Action::SyncProgress {
                    current: current_block,
                    target: sync_until,
                });

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

        // Report synced state with current position
        let _ = action_tx.send(Action::SyncProgress {
            current: current_block.saturating_sub(1),
            target: sync_until,
        });
        let _ = action_tx.send(Action::SyncDone);

        // If we've reached the stop block, we're done syncing
        if let Some(stop) = stop_block {
            if current_block > stop {
                return Ok(());
            }
        }
    }
}
