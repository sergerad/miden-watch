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

    // For Tip mode, cache covers all stored blocks (not just from the current tip).
    // Use a reference here so start_from is not consumed before the current_block match.
    let cache_load_from = match &start_from {
        StartFrom::Block(n) => *n,
        StartFrom::Genesis | StartFrom::Tip => 0,
    };

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

    // Load cached blocks from the store into the UI in batches
    {
        const BATCH_SIZE: u32 = 500;
        let store = store.lock().await;
        let total = store
            .count_blocks_in_range(cache_load_from, stop_block)
            .unwrap_or(0);

        if total > 0 {
            let mut loaded: u32 = 0;
            let mut offset: u32 = 0;
            let load_from = cache_load_from;

            loop {
                let blocks = store
                    .get_blocks_page(load_from, stop_block, BATCH_SIZE, offset)
                    .unwrap_or_default();
                if blocks.is_empty() {
                    break;
                }

                // Get block num range for batch tx/note loading
                let batch_min = blocks.first().unwrap().block_num;
                let batch_max = blocks.last().unwrap().block_num;

                let all_txs = store
                    .get_transactions_for_blocks(batch_min, batch_max)
                    .unwrap_or_default();
                let all_notes = store
                    .get_notes_for_blocks(batch_min, batch_max)
                    .unwrap_or_default();

                // Group txs and notes by block_num
                let mut txs_by_block: std::collections::HashMap<u32, Vec<_>> =
                    std::collections::HashMap::new();
                for tx in all_txs {
                    txs_by_block.entry(tx.block_num).or_default().push(tx);
                }
                let mut notes_by_block: std::collections::HashMap<u32, Vec<_>> =
                    std::collections::HashMap::new();
                for note in all_notes {
                    notes_by_block.entry(note.block_num).or_default().push(note);
                }

                let batch_len = blocks.len() as u32;
                loaded += batch_len;
                let _ = action_tx.send(Action::LoadProgress {
                    current: loaded,
                    target: total,
                });

                let mut batch = Vec::with_capacity(blocks.len());
                for block in blocks {
                    let block_num = block.block_num;
                    let txs = txs_by_block.remove(&block_num).unwrap_or_default();
                    let notes = notes_by_block.remove(&block_num).unwrap_or_default();
                    if block_num == current_block {
                        current_block = block_num + 1;
                    }
                    batch.push((block, txs, notes));
                }
                let _ = action_tx.send(Action::BatchBlocksLoaded { blocks: batch });

                offset += batch_len;
                if batch_len < BATCH_SIZE {
                    break;
                }
            }
        }
    }

    // For Tip mode: resume from where we left off rather than jumping to the current tip,
    // so blocks that arrived between runs are not skipped.
    {
        let store = store.lock().await;
        if let Ok(Some(latest)) = store.get_latest_block_num() {
            let resume = latest + 1;
            if resume < current_block {
                current_block = resume;
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

                let fetch_start = std::time::Instant::now();
                match rpc.fetch_block_data(current_block).await {
                    Ok(sync_result) => {
                        let latency = fetch_start.elapsed().as_millis() as u64;
                        let _ = action_tx.send(Action::Latency(latency));

                        let store = store.lock().await;
                        if let Err(e) = store.insert_block(&sync_result.block) {
                            let _ = action_tx.send(Action::SyncError(format!(
                                "Failed to store block {}: {e}",
                                current_block
                            )));
                        }
                        if let Err(e) = store.insert_transactions(&sync_result.transactions) {
                            let _ = action_tx.send(Action::SyncError(format!(
                                "Failed to store txs for block {}: {e}",
                                current_block
                            )));
                        }
                        if let Err(e) = store.insert_notes(&sync_result.notes) {
                            let _ = action_tx.send(Action::SyncError(format!(
                                "Failed to store notes for block {}: {e}",
                                current_block
                            )));
                        }
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
