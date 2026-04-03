use crate::types::{BlockInfo, NoteInfo, TransactionInfo};

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Quit,
    Up,
    Down,
    Enter,
    Back,
    ToggleTailing,
    NewBlockReceived {
        block: BlockInfo,
        transactions: Vec<TransactionInfo>,
        notes: Vec<NoteInfo>,
    },
    SyncError(String),
    ScrollUp,
    ScrollDown,
    HalfPageUp,
    HalfPageDown,
    GoToTop,
    GoToBottom,
    ToggleHelp,
    NavBack,
    NavForward,
}
