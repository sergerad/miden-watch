use std::collections::HashMap;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const HALF_PAGE: usize = 15;
use ratatui::widgets::ListState;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::event::Event;
use crate::types::{BlockInfo, NoteInfo, TransactionInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFilter {
    All,
    WithTransactions,
    WithNotes,
}

impl BlockFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::WithTransactions,
            Self::WithTransactions => Self::WithNotes,
            Self::WithNotes => Self::All,
        }
    }
}

impl std::fmt::Display for BlockFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "All"),
            Self::WithTransactions => write!(f, "Has txs"),
            Self::WithNotes => write!(f, "Has notes"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    BlockList,
    BlockDetail,
    TxDetail,
    NoteDetail,
}

/// Which sub-panel has focus within the BlockDetail view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailFocus {
    Block,
    Txs,
    Notes,
}

impl DetailFocus {
    pub fn next(self) -> Self {
        match self {
            Self::Block => Self::Txs,
            Self::Txs => Self::Notes,
            Self::Notes => Self::Block,
        }
    }
}

#[derive(Debug, Clone)]
struct NavEntry {
    pane: Pane,
    browsing_block_num: Option<u32>,
    block_list_selected: Option<usize>,
    tx_list_selected: Option<usize>,
    note_list_selected: Option<usize>,
    detail_scroll: u16,
    detail_focus: DetailFocus,
    detail_row_selected: Option<usize>,
}

pub struct App {
    pub active_pane: Pane,
    pub blocks: Vec<BlockInfo>,
    pub block_list_state: ListState,
    pub block_transactions: HashMap<u32, Vec<TransactionInfo>>,
    pub block_notes: HashMap<u32, Vec<NoteInfo>>,
    /// The block number currently being browsed (pinned across new block arrivals)
    pub browsing_block_num: Option<u32>,
    pub selected_block_txs: Vec<TransactionInfo>,
    pub selected_block_notes: Vec<NoteInfo>,
    pub tx_list_state: ListState,
    pub note_list_state: ListState,
    pub error_log: Vec<String>,
    pub error_list_state: ListState,
    pub show_error_log: bool,
    pub should_quit: bool,
    #[allow(dead_code)]
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub detail_scroll: u16,
    /// Tracks a pending 'g' keypress for the gg sequence
    pending_g: bool,
    /// Numeric prefix for vim-style count (e.g. 200j)
    count_buf: String,
    pub show_help: bool,
    /// Navigation history for Ctrl+o / Ctrl+i
    nav_back: Vec<NavEntry>,
    nav_forward: Vec<NavEntry>,
    /// Sync progress for status bar display
    pub sync_progress: Option<(u32, u32)>,
    /// Search input state: when Some, the user is typing a block number
    pub search_input: Option<String>,
    /// Active block list filter
    pub filter: BlockFilter,
    /// Block number and time of the most recently received block (for flash highlight)
    pub last_received_block: Option<(u32, Instant)>,
    /// Loading from DB progress
    pub load_progress: Option<(u32, u32)>,
    /// Whether sync has caught up to the target
    pub sync_done: bool,
    /// Network name derived from the RPC URL
    pub network: String,
    /// Last measured RPC latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Whether to show full hex hashes or truncated
    pub expand_hashes: bool,
    /// Which sub-panel has focus in BlockDetail
    pub detail_focus: DetailFocus,
    /// Clipboard status message (shown briefly)
    pub clipboard_flash: Option<(String, Instant)>,
    /// Row selection state for detail views (TxDetail, NoteDetail)
    pub detail_row_state: ListState,
    /// The key-value rows currently displayed in the detail view (for copy)
    pub detail_rows: Vec<(String, String)>,
}

impl App {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>, network: String) -> Self {
        Self {
            active_pane: Pane::BlockList,
            blocks: Vec::new(),
            block_list_state: ListState::default(),
            block_transactions: HashMap::new(),
            block_notes: HashMap::new(),
            browsing_block_num: None,
            selected_block_txs: Vec::new(),
            selected_block_notes: Vec::new(),
            tx_list_state: ListState::default(),
            note_list_state: ListState::default(),
            error_log: Vec::new(),
            error_list_state: ListState::default(),
            show_error_log: false,
            should_quit: false,
            action_tx,
            detail_scroll: 0,
            pending_g: false,
            count_buf: String::new(),
            show_help: false,
            nav_back: Vec::new(),
            nav_forward: Vec::new(),
            sync_progress: None,
            search_input: None,
            filter: BlockFilter::All,
            last_received_block: None,
            load_progress: None,
            sync_done: false,
            network,
            latency_ms: None,
            expand_hashes: false,
            detail_focus: DetailFocus::Block,
            clipboard_flash: None,
            detail_row_state: ListState::default(),
            detail_rows: Vec::new(),
        }
    }

    pub fn handle_event(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Tick => Some(Action::Tick),
            Event::Resize(_, _) => None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(Action::Quit);
        }

        // When help is open, any key closes it
        if self.show_help {
            return Some(Action::ToggleHelp);
        }

        // When error log is open, handle its navigation or close it
        if self.show_error_log {
            return match key.code {
                KeyCode::Char('!') | KeyCode::Esc => Some(Action::ToggleErrorLog),
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Char('j') | KeyCode::Down => Some(Action::Down(1)),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Up(1)),
                KeyCode::Char('c') => Some(Action::ClearErrorLog),
                _ => None,
            };
        }

        // When search input is active, handle typing
        if let Some(ref mut input) = self.search_input {
            match key.code {
                KeyCode::Esc => {
                    self.search_input = None;
                    return None;
                }
                KeyCode::Enter => {
                    let result = input.parse::<u32>().ok();
                    self.search_input = None;
                    return result.map(Action::SearchBlock);
                }
                KeyCode::Backspace => {
                    input.pop();
                    if input.is_empty() {
                        self.search_input = None;
                    }
                    return None;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    input.push(c);
                    return None;
                }
                _ => return None,
            }
        }

        // ? opens help
        if key.code == KeyCode::Char('?') {
            return Some(Action::ToggleHelp);
        }

        // ! opens error log
        if key.code == KeyCode::Char('!') {
            return Some(Action::ToggleErrorLog);
        }

        // Ctrl key combinations
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('u') => {
                    self.pending_g = false;
                    return Some(Action::HalfPageUp);
                }
                KeyCode::Char('d') => {
                    self.pending_g = false;
                    return Some(Action::HalfPageDown);
                }
                KeyCode::Char('o') => {
                    self.pending_g = false;
                    return Some(Action::NavBack);
                }
                KeyCode::Char('i') => {
                    self.pending_g = false;
                    return Some(Action::NavForward);
                }
                _ => {}
            }
        }

        // Tab: switch focus in BlockDetail, nav forward elsewhere
        if key.code == KeyCode::Tab {
            self.pending_g = false;
            if self.active_pane == Pane::BlockDetail {
                return Some(Action::SwitchDetailFocus);
            }
            return Some(Action::NavForward);
        }

        // Handle gg / G sequences
        if key.code == KeyCode::Char('g') && !key.modifiers.contains(KeyModifiers::SHIFT) {
            if self.pending_g {
                self.pending_g = false;
                return Some(Action::GoToTop);
            } else {
                self.pending_g = true;
                return None;
            }
        }
        if key.code == KeyCode::Char('G') {
            self.pending_g = false;
            return Some(Action::GoToBottom);
        }
        // Any other key cancels a pending g
        self.pending_g = false;

        // Accumulate digits for vim-style count prefix (e.g. 200j)
        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && !key.modifiers.contains(KeyModifiers::CONTROL) {
                self.count_buf.push(c);
                return None;
            }
        }

        let count = self.take_count();

        match self.active_pane {
            Pane::BlockList => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Char('j') | KeyCode::Down => Some(Action::Down(count)),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Up(count)),
                KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => Some(Action::Enter),
                KeyCode::Char('/') => {
                    self.search_input = Some(String::new());
                    None
                }
                KeyCode::Char('f') => Some(Action::CycleFilter),
                _ => None,
            },
            Pane::BlockDetail => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                    Some(Action::Back)
                }
                KeyCode::Char('j') | KeyCode::Down => Some(Action::Down(count)),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Up(count)),
                KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => Some(Action::Enter),
                KeyCode::Char('e') => Some(Action::ToggleExpandHashes),
                KeyCode::Char('y') => Some(Action::CopyHash),
                _ => None,
            },
            Pane::TxDetail => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                    Some(Action::Back)
                }
                KeyCode::Char('j') | KeyCode::Down => Some(Action::Down(count)),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Up(count)),
                KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => Some(Action::Enter),
                KeyCode::Char('e') => Some(Action::ToggleExpandHashes),
                KeyCode::Char('y') => Some(Action::CopyHash),
                _ => None,
            },
            Pane::NoteDetail => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                    Some(Action::Back)
                }
                KeyCode::Char('j') | KeyCode::Down => Some(Action::Down(count)),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Up(count)),
                KeyCode::Char('e') => Some(Action::ToggleExpandHashes),
                KeyCode::Char('y') => Some(Action::CopyHash),
                _ => None,
            },
        }
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Tick => {}
            Action::ToggleErrorLog => {
                self.show_error_log = !self.show_error_log;
                if self.show_error_log && !self.error_log.is_empty() {
                    self.error_list_state.select(Some(self.error_log.len() - 1));
                }
            }
            Action::ClearErrorLog => {
                self.error_log.clear();
                self.error_list_state.select(None);
                self.show_error_log = false;
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::Up(n) => {
                if self.show_error_log {
                    self.select_prev_error(n);
                } else {
                    match self.active_pane {
                        Pane::BlockList => {
                            if n > 1 {
                                self.push_nav();
                            }
                            self.select_prev_block(n);
                        }
                        Pane::BlockDetail => match self.detail_focus {
                            DetailFocus::Block => self.select_prev_detail_row(n),
                            DetailFocus::Txs => self.select_prev_tx(n),
                            DetailFocus::Notes => self.select_prev_note(n),
                        },
                        Pane::TxDetail | Pane::NoteDetail => {
                            self.select_prev_detail_row(n);
                        }
                    }
                }
            }
            Action::Down(n) => {
                if self.show_error_log {
                    self.select_next_error(n);
                } else {
                    match self.active_pane {
                        Pane::BlockList => {
                            if n > 1 {
                                self.push_nav();
                            }
                            self.select_next_block(n);
                        }
                        Pane::BlockDetail => match self.detail_focus {
                            DetailFocus::Block => self.select_next_detail_row(n),
                            DetailFocus::Txs => self.select_next_tx(n),
                            DetailFocus::Notes => self.select_next_note(n),
                        },
                        Pane::TxDetail | Pane::NoteDetail => {
                            self.select_next_detail_row(n);
                        }
                    }
                }
            }
            Action::Enter => match self.active_pane {
                Pane::BlockList => {
                    let filtered = self.filtered_indices();
                    if let Some(fi) = self.block_list_state.selected() {
                        if let Some(&real_idx) = filtered.get(fi) {
                            self.push_nav();
                            let block_num = self.blocks[real_idx].block_num;
                            self.browse_block(block_num);
                            self.active_pane = Pane::BlockDetail;
                        }
                    }
                }
                Pane::BlockDetail => match self.detail_focus {
                    DetailFocus::Block => {
                        // Block info focused — no drill-down, but could copy
                    }
                    DetailFocus::Notes => {
                        if let Some(idx) = self.note_list_state.selected() {
                            if idx < self.selected_block_notes.len() {
                                self.push_nav();
                                self.enter_detail_view(Pane::NoteDetail);
                            }
                        }
                    }
                    DetailFocus::Txs => {
                        if let Some(idx) = self.tx_list_state.selected() {
                            if idx < self.selected_block_txs.len() {
                                self.push_nav();
                                self.enter_detail_view(Pane::TxDetail);
                            }
                        }
                    }
                },
                Pane::TxDetail => {
                    // Enter in TxDetail: navigate to NoteDetail if there are notes
                    if !self.selected_block_notes.is_empty() {
                        self.push_nav();
                        if self.note_list_state.selected().is_none() {
                            self.note_list_state.select(Some(0));
                        }
                        self.enter_detail_view(Pane::NoteDetail);
                    }
                }
                _ => {}
            },
            Action::Back => match self.active_pane {
                Pane::BlockDetail => {
                    self.push_nav();
                    self.browsing_block_num = None;
                    self.active_pane = Pane::BlockList;
                }
                Pane::TxDetail | Pane::NoteDetail => {
                    self.push_nav();
                    self.active_pane = Pane::BlockDetail;
                    self.detail_scroll = 0;
                }
                Pane::BlockList => {}
            },
            Action::NavBack => {
                if let Some(entry) = self.nav_back.pop() {
                    self.nav_forward.push(self.nav_snapshot());
                    self.restore_nav(entry);
                }
            }
            Action::NavForward => {
                if let Some(entry) = self.nav_forward.pop() {
                    self.nav_back.push(self.nav_snapshot());
                    self.restore_nav(entry);
                }
            }
            Action::NewBlockReceived {
                block,
                transactions,
                notes,
            } => {
                let at_head = self.block_list_state.selected() == Some(0);
                let selected_bn = self.selected_block_num_filtered();
                let block_num = block.block_num;
                self.insert_block(block, transactions, notes);
                self.last_received_block = Some((block_num, Instant::now()));

                let filtered = self.filtered_indices();
                if at_head || filtered.len() <= 1 {
                    self.block_list_state.select(Some(0));
                } else if let Some(bn) = selected_bn {
                    if let Some(fi) = filtered
                        .iter()
                        .position(|&ri| self.blocks[ri].block_num == bn)
                    {
                        self.block_list_state.select(Some(fi));
                    }
                }
            }
            Action::BatchBlocksLoaded { blocks } => {
                let at_head = self.block_list_state.selected() == Some(0)
                    || self.block_list_state.selected().is_none();
                let selected_bn = self.selected_block_num_filtered();

                for (block, transactions, notes) in blocks {
                    self.insert_block(block, transactions, notes);
                }

                let filtered = self.filtered_indices();
                if at_head || filtered.len() <= 1 {
                    self.block_list_state.select(Some(0));
                } else if let Some(bn) = selected_bn {
                    if let Some(fi) = filtered
                        .iter()
                        .position(|&ri| self.blocks[ri].block_num == bn)
                    {
                        self.block_list_state.select(Some(fi));
                    }
                }
            }
            Action::SyncError(msg) => {
                self.error_log.push(msg);
            }
            Action::HalfPageUp => match self.active_pane {
                Pane::BlockList => {
                    let flen = self.filtered_indices().len();
                    if flen > 0 {
                        self.push_nav();
                        let current = self.block_list_state.selected().unwrap_or(0);
                        let target = current.saturating_sub(HALF_PAGE);
                        self.block_list_state.select(Some(target));
                    }
                }
                Pane::BlockDetail => match self.detail_focus {
                    DetailFocus::Block => self.select_prev_detail_row(HALF_PAGE),
                    DetailFocus::Txs => {
                        let current = self.tx_list_state.selected().unwrap_or(0);
                        let target = current.saturating_sub(HALF_PAGE);
                        self.tx_list_state.select(Some(target));
                    }
                    DetailFocus::Notes => {
                        let current = self.note_list_state.selected().unwrap_or(0);
                        let target = current.saturating_sub(HALF_PAGE);
                        self.note_list_state.select(Some(target));
                    }
                },
                Pane::TxDetail | Pane::NoteDetail => {
                    self.select_prev_detail_row(HALF_PAGE);
                }
            },
            Action::HalfPageDown => match self.active_pane {
                Pane::BlockList => {
                    let flen = self.filtered_indices().len();
                    if flen > 0 {
                        self.push_nav();
                        let current = self.block_list_state.selected().unwrap_or(0);
                        let target = (current + HALF_PAGE).min(flen - 1);
                        self.block_list_state.select(Some(target));
                    }
                }
                Pane::BlockDetail => match self.detail_focus {
                    DetailFocus::Block => self.select_next_detail_row(HALF_PAGE),
                    DetailFocus::Txs => {
                        if !self.selected_block_txs.is_empty() {
                            let current = self.tx_list_state.selected().unwrap_or(0);
                            let target =
                                (current + HALF_PAGE).min(self.selected_block_txs.len() - 1);
                            self.tx_list_state.select(Some(target));
                        }
                    }
                    DetailFocus::Notes => {
                        if !self.selected_block_notes.is_empty() {
                            let current = self.note_list_state.selected().unwrap_or(0);
                            let target =
                                (current + HALF_PAGE).min(self.selected_block_notes.len() - 1);
                            self.note_list_state.select(Some(target));
                        }
                    }
                },
                Pane::TxDetail | Pane::NoteDetail => {
                    self.select_next_detail_row(HALF_PAGE);
                }
            },
            Action::GoToTop => match self.active_pane {
                Pane::BlockList => {
                    if !self.filtered_indices().is_empty() {
                        self.push_nav();
                        self.block_list_state.select(Some(0));
                    }
                }
                Pane::BlockDetail => match self.detail_focus {
                    DetailFocus::Block => {
                        if !self.detail_rows.is_empty() {
                            self.detail_row_state.select(Some(0));
                        }
                    }
                    DetailFocus::Txs => {
                        if !self.selected_block_txs.is_empty() {
                            self.tx_list_state.select(Some(0));
                        }
                    }
                    DetailFocus::Notes => {
                        if !self.selected_block_notes.is_empty() {
                            self.note_list_state.select(Some(0));
                        }
                    }
                },
                Pane::TxDetail | Pane::NoteDetail => {
                    if !self.detail_rows.is_empty() {
                        self.detail_row_state.select(Some(0));
                    }
                }
            },
            Action::GoToBottom => match self.active_pane {
                Pane::BlockList => {
                    let flen = self.filtered_indices().len();
                    if flen > 0 {
                        self.push_nav();
                        self.block_list_state.select(Some(flen - 1));
                    }
                }
                Pane::BlockDetail => match self.detail_focus {
                    DetailFocus::Block => {
                        if !self.detail_rows.is_empty() {
                            self.detail_row_state
                                .select(Some(self.detail_rows.len() - 1));
                        }
                    }
                    DetailFocus::Txs => {
                        if !self.selected_block_txs.is_empty() {
                            self.tx_list_state
                                .select(Some(self.selected_block_txs.len() - 1));
                        }
                    }
                    DetailFocus::Notes => {
                        if !self.selected_block_notes.is_empty() {
                            self.note_list_state
                                .select(Some(self.selected_block_notes.len() - 1));
                        }
                    }
                },
                Pane::TxDetail | Pane::NoteDetail => {
                    if !self.detail_rows.is_empty() {
                        self.detail_row_state
                            .select(Some(self.detail_rows.len() - 1));
                    }
                }
            },
            Action::SearchBlock(block_num) => {
                self.push_nav();
                let filtered = self.filtered_indices();
                // Find the closest block in filtered view: exact match or nearest lower
                if let Some(fi) = filtered
                    .iter()
                    .position(|&ri| self.blocks[ri].block_num <= block_num)
                {
                    self.block_list_state.select(Some(fi));
                }
            }
            Action::CycleFilter => {
                self.filter = self.filter.next();
                let filtered = self.filtered_indices();
                if filtered.is_empty() {
                    self.block_list_state.select(None);
                } else {
                    self.block_list_state.select(Some(0));
                }
            }
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            Action::LoadProgress { current, target } => {
                self.load_progress = Some((current, target));
            }
            Action::SyncProgress { current, target } => {
                self.load_progress = None;
                self.sync_progress = Some((current, target));
                self.sync_done = false;
            }
            Action::SyncDone => {
                self.sync_done = true;
            }
            Action::Latency(ms) => {
                self.latency_ms = Some(ms);
            }
            Action::ToggleExpandHashes => {
                self.expand_hashes = !self.expand_hashes;
            }
            Action::CopyHash => {
                self.copy_selected_hash();
            }
            Action::SwitchDetailFocus => {
                if self.active_pane == Pane::BlockDetail {
                    self.detail_focus = self.detail_focus.next();
                    match self.detail_focus {
                        DetailFocus::Block => {
                            // Initialize block row selection
                            if self.detail_row_state.selected().is_none()
                                && !self.detail_rows.is_empty()
                            {
                                self.detail_row_state.select(Some(0));
                            }
                        }
                        DetailFocus::Txs => {
                            if self.tx_list_state.selected().is_none()
                                && !self.selected_block_txs.is_empty()
                            {
                                self.tx_list_state.select(Some(0));
                            }
                        }
                        DetailFocus::Notes => {
                            if self.note_list_state.selected().is_none()
                                && !self.selected_block_notes.is_empty()
                            {
                                self.note_list_state.select(Some(0));
                            }
                        }
                    }
                }
            }
        }
    }

    fn take_count(&mut self) -> usize {
        if self.count_buf.is_empty() {
            return 1;
        }
        let count = self.count_buf.parse::<usize>().unwrap_or(1).max(1);
        self.count_buf.clear();
        count
    }

    fn nav_snapshot(&self) -> NavEntry {
        NavEntry {
            pane: self.active_pane,
            browsing_block_num: self.browsing_block_num,
            block_list_selected: self.block_list_state.selected(),
            tx_list_selected: self.tx_list_state.selected(),
            note_list_selected: self.note_list_state.selected(),
            detail_scroll: self.detail_scroll,
            detail_focus: self.detail_focus,
            detail_row_selected: self.detail_row_state.selected(),
        }
    }

    fn push_nav(&mut self) {
        self.nav_back.push(self.nav_snapshot());
        self.nav_forward.clear();
    }

    fn restore_nav(&mut self, entry: NavEntry) {
        self.active_pane = entry.pane;
        self.browsing_block_num = entry.browsing_block_num;
        self.block_list_state.select(entry.block_list_selected);
        self.detail_scroll = entry.detail_scroll;
        self.detail_focus = entry.detail_focus;

        // Restore detail view data if we're going back to a block detail/tx/note view
        if let Some(bn) = entry.browsing_block_num {
            self.selected_block_txs = self
                .block_transactions
                .get(&bn)
                .cloned()
                .unwrap_or_default();
            self.selected_block_notes = self.block_notes.get(&bn).cloned().unwrap_or_default();
            self.tx_list_state = ListState::default();
            self.tx_list_state.select(entry.tx_list_selected);
            self.note_list_state = ListState::default();
            self.note_list_state.select(entry.note_list_selected);
        }
        self.detail_row_state = ListState::default();
        self.detail_row_state.select(entry.detail_row_selected);
    }

    /// Pin the detail view to a specific block number
    fn browse_block(&mut self, block_num: u32) {
        self.browsing_block_num = Some(block_num);
        self.selected_block_txs = self
            .block_transactions
            .get(&block_num)
            .cloned()
            .unwrap_or_default();
        self.selected_block_notes = self
            .block_notes
            .get(&block_num)
            .cloned()
            .unwrap_or_default();
        self.tx_list_state = ListState::default();
        self.note_list_state = ListState::default();
        self.detail_focus = DetailFocus::Block;
        self.detail_row_state = ListState::default();
        self.detail_row_state.select(Some(0));
        self.detail_rows.clear();
        if !self.selected_block_txs.is_empty() {
            self.tx_list_state.select(Some(0));
        }
        if !self.selected_block_notes.is_empty() {
            self.note_list_state.select(Some(0));
        }
    }

    fn insert_block(
        &mut self,
        block: BlockInfo,
        transactions: Vec<TransactionInfo>,
        notes: Vec<NoteInfo>,
    ) {
        let block_num = block.block_num;
        let mut updated_block = block;
        updated_block.tx_count = transactions.len();
        updated_block.note_count = notes.len();

        // Insert in sorted position (newest first)
        let pos = self
            .blocks
            .iter()
            .position(|b| b.block_num <= block_num)
            .unwrap_or(self.blocks.len());

        // Avoid duplicates
        if pos < self.blocks.len() && self.blocks[pos].block_num == block_num {
            self.blocks[pos] = updated_block;
        } else {
            self.blocks.insert(pos, updated_block);
        }

        self.block_transactions.insert(block_num, transactions);
        self.block_notes.insert(block_num, notes);
    }

    fn select_next_block(&mut self, n: usize) {
        let flen = self.filtered_indices().len();
        if flen == 0 {
            return;
        }
        let i = match self.block_list_state.selected() {
            Some(i) => (i + n).min(flen - 1),
            None => 0,
        };
        self.block_list_state.select(Some(i));
    }

    fn select_prev_block(&mut self, n: usize) {
        let flen = self.filtered_indices().len();
        if flen == 0 {
            return;
        }
        let i = match self.block_list_state.selected() {
            Some(i) => i.saturating_sub(n),
            None => 0,
        };
        self.block_list_state.select(Some(i));
    }

    fn select_next_tx(&mut self, n: usize) {
        if self.selected_block_txs.is_empty() {
            return;
        }
        let i = match self.tx_list_state.selected() {
            Some(i) => (i + n).min(self.selected_block_txs.len() - 1),
            None => 0,
        };
        self.tx_list_state.select(Some(i));
    }

    fn select_prev_tx(&mut self, n: usize) {
        if self.selected_block_txs.is_empty() {
            return;
        }
        let i = match self.tx_list_state.selected() {
            Some(i) => i.saturating_sub(n),
            None => 0,
        };
        self.tx_list_state.select(Some(i));
    }

    fn select_next_error(&mut self, n: usize) {
        if self.error_log.is_empty() {
            return;
        }
        let i = match self.error_list_state.selected() {
            Some(i) => (i + n).min(self.error_log.len() - 1),
            None => 0,
        };
        self.error_list_state.select(Some(i));
    }

    fn select_prev_error(&mut self, n: usize) {
        if self.error_log.is_empty() {
            return;
        }
        let i = match self.error_list_state.selected() {
            Some(i) => i.saturating_sub(n),
            None => 0,
        };
        self.error_list_state.select(Some(i));
    }

    /// Get the block number currently selected in the filtered block list
    fn selected_block_num_filtered(&self) -> Option<u32> {
        let filtered = self.filtered_indices();
        self.block_list_state
            .selected()
            .and_then(|fi| filtered.get(fi).copied())
            .and_then(|ri| self.blocks.get(ri))
            .map(|b| b.block_num)
    }

    /// Returns indices into `self.blocks` that match the current filter
    pub fn filtered_indices(&self) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| match self.filter {
                BlockFilter::All => true,
                BlockFilter::WithTransactions => b.tx_count > 0,
                BlockFilter::WithNotes => b.note_count > 0,
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Returns the block currently being browsed (pinned), or the list-selected block
    pub fn selected_block(&self) -> Option<&BlockInfo> {
        if let Some(bn) = self.browsing_block_num {
            self.blocks.iter().find(|b| b.block_num == bn)
        } else {
            self.block_list_state
                .selected()
                .and_then(|i| self.blocks.get(i))
        }
    }

    pub fn selected_tx(&self) -> Option<&TransactionInfo> {
        self.tx_list_state
            .selected()
            .and_then(|i| self.selected_block_txs.get(i))
    }

    pub fn selected_note(&self) -> Option<&NoteInfo> {
        self.note_list_state
            .selected()
            .and_then(|i| self.selected_block_notes.get(i))
    }

    fn select_next_note(&mut self, n: usize) {
        if self.selected_block_notes.is_empty() {
            return;
        }
        let i = match self.note_list_state.selected() {
            Some(i) => (i + n).min(self.selected_block_notes.len() - 1),
            None => 0,
        };
        self.note_list_state.select(Some(i));
    }

    fn select_prev_note(&mut self, n: usize) {
        if self.selected_block_notes.is_empty() {
            return;
        }
        let i = match self.note_list_state.selected() {
            Some(i) => i.saturating_sub(n),
            None => 0,
        };
        self.note_list_state.select(Some(i));
    }

    /// Enter a detail view (TxDetail or NoteDetail), resetting row selection
    fn enter_detail_view(&mut self, pane: Pane) {
        self.active_pane = pane;
        self.detail_row_state = ListState::default();
        self.detail_row_state.select(Some(0));
        self.detail_rows.clear();
    }

    fn select_next_detail_row(&mut self, n: usize) {
        if self.detail_rows.is_empty() {
            return;
        }
        let i = match self.detail_row_state.selected() {
            Some(i) => (i + n).min(self.detail_rows.len() - 1),
            None => 0,
        };
        self.detail_row_state.select(Some(i));
    }

    fn select_prev_detail_row(&mut self, n: usize) {
        if self.detail_rows.is_empty() {
            return;
        }
        let i = match self.detail_row_state.selected() {
            Some(i) => i.saturating_sub(n),
            None => 0,
        };
        self.detail_row_state.select(Some(i));
    }

    fn copy_selected_hash(&mut self) {
        let value = match self.active_pane {
            Pane::BlockDetail => match self.detail_focus {
                DetailFocus::Block => self
                    .detail_row_state
                    .selected()
                    .and_then(|i| self.detail_rows.get(i))
                    .map(|(_, v)| v.clone()),
                DetailFocus::Txs => self.selected_tx().map(|t| t.tx_id.clone()),
                DetailFocus::Notes => self.selected_note().map(|n| n.note_id.clone()),
            },
            Pane::TxDetail | Pane::NoteDetail => {
                // Copy the value of the highlighted row
                self.detail_row_state
                    .selected()
                    .and_then(|i| self.detail_rows.get(i))
                    .map(|(_, v)| v.clone())
            }
            _ => None,
        };

        if let Some(value) = value {
            if let Ok(mut child) = std::process::Command::new("pbcopy")
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(value.as_bytes());
                }
                let _ = child.wait();
                self.clipboard_flash = Some(("Copied!".to_string(), Instant::now()));
            }
        }
    }
}
