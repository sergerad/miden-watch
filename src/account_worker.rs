use std::sync::Arc;

use miden_client::rpc::Endpoint;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::rpc::RpcClient;
use crate::store::Store;
use crate::types::AccountInfo;

/// Long-lived task that services on-demand account lookups.
///
/// Mirrors `sync::run_sync`: owns its own [`RpcClient`], shares the `Arc<Mutex<Store>>`, receives
/// account-id requests on `req_rx`, and pushes results back through `action_tx`. For each request
/// it loads local history from the store (fast) and then fetches live public state from the node,
/// bundling both into one [`Action::AccountViewLoaded`] so the UI re-renders once.
pub async fn run(
    endpoint: Endpoint,
    store: Arc<Mutex<Store>>,
    mut req_rx: mpsc::UnboundedReceiver<String>,
    action_tx: mpsc::UnboundedSender<Action>,
) {
    let rpc = RpcClient::new(&endpoint);

    while let Some(mut id) = req_rx.recv().await {
        // Coalesce: if the user fired several lookups in quick succession, only serve the last.
        while let Ok(newer) = req_rx.try_recv() {
            id = newer;
        }

        // 1) Local history from the store (do not hold the lock across the RPC await).
        let (txs, sent_notes, received_notes) = {
            let store = store.lock().await;
            (
                store.get_transactions_for_account(&id).unwrap_or_default(),
                store.get_notes_by_sender(&id).unwrap_or_default(),
                store.get_notes_by_target(&id).unwrap_or_default(),
            )
        };

        // 2) Live public state from the node.
        let (account_type, is_public, live_state, error) = match rpc.get_account_view(&id).await {
            Ok(tuple) => tuple,
            Err(e) => (String::new(), false, None, Some(format!("{e}"))),
        };

        let info = AccountInfo {
            account_id: id,
            account_type,
            is_public,
            live_state,
            error,
            txs,
            sent_notes,
            received_notes,
        };

        let _ = action_tx.send(Action::AccountViewLoaded(Box::new(info)));
    }
}
