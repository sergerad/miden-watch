use crate::types::{BlockInfo, NoteInfo, TransactionInfo};

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Quit,
    Up(usize),
    Down(usize),
    Enter,
    Back,
    NewBlockReceived {
        block: BlockInfo,
        transactions: Vec<TransactionInfo>,
        notes: Vec<NoteInfo>,
    },
    BatchBlocksLoaded {
        blocks: Vec<(BlockInfo, Vec<TransactionInfo>, Vec<NoteInfo>)>,
    },
    SyncError(String),
    ScrollUp,
    ScrollDown,
    HalfPageUp,
    HalfPageDown,
    GoToTop,
    GoToBottom,
    ToggleHelp,
    ToggleErrorLog,
    ClearErrorLog,
    SearchBlock(u32),
    CycleFilter,
    NavBack,
    NavForward,
    LoadProgress {
        current: u32,
        target: u32,
    },
    SyncProgress {
        current: u32,
        target: u32,
    },
    SyncDone,
    Latency(u64),
}
