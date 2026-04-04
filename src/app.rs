use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const HALF_PAGE: usize = 15;
use ratatui::widgets::ListState;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::event::Event;
use crate::types::{BlockInfo, NoteInfo, TransactionInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Tailing,
    Browsing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    BlockList,
    BlockDetail,
    TxDetail,
    NoteDetail,
}

#[derive(Debug, Clone)]
struct NavEntry {
    pane: Pane,
    browsing_block_num: Option<u32>,
    block_list_selected: Option<usize>,
    tx_list_selected: Option<usize>,
    detail_scroll: u16,
}

pub struct App {
    pub mode: Mode,
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
    pub error: Option<String>,
    pub error_time: Option<std::time::Instant>,
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
}

impl App {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>) -> Self {
        Self {
            mode: Mode::Tailing,
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
            error: None,
            error_time: None,
            should_quit: false,
            action_tx,
            detail_scroll: 0,
            pending_g: false,
            count_buf: String::new(),
            show_help: false,
            nav_back: Vec::new(),
            nav_forward: Vec::new(),
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

        // ? opens help
        if key.code == KeyCode::Char('?') {
            return Some(Action::ToggleHelp);
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

        // Tab is Ctrl+I in most terminals — nav forward
        if key.code == KeyCode::Tab {
            self.pending_g = false;
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
                KeyCode::Char('t') => Some(Action::ToggleTailing),
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
                KeyCode::Char('t') => Some(Action::ToggleTailing),
                _ => None,
            },
            Pane::TxDetail => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                    Some(Action::Back)
                }
                KeyCode::Char('j') | KeyCode::Down => Some(Action::ScrollDown),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::ScrollUp),
                KeyCode::Char('t') => Some(Action::ToggleTailing),
                _ => None,
            },
            Pane::NoteDetail => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                    Some(Action::Back)
                }
                KeyCode::Char('j') | KeyCode::Down => Some(Action::ScrollDown),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::ScrollUp),
                KeyCode::Char('t') => Some(Action::ToggleTailing),
                _ => None,
            },
        }
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Tick => {
                // Clear errors after 10 seconds
                if let Some(time) = self.error_time {
                    if time.elapsed().as_secs() > 10 {
                        self.error = None;
                        self.error_time = None;
                    }
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::Up(n) => match self.active_pane {
                Pane::BlockList => {
                    if n > 1 {
                        self.push_nav();
                    }
                    self.select_prev_block(n);
                }
                Pane::BlockDetail => self.select_prev_tx(n),
                _ => {}
            },
            Action::Down(n) => match self.active_pane {
                Pane::BlockList => {
                    if n > 1 {
                        self.push_nav();
                    }
                    self.select_next_block(n);
                }
                Pane::BlockDetail => self.select_next_tx(n),
                _ => {}
            },
            Action::Enter => match self.active_pane {
                Pane::BlockList => {
                    if let Some(idx) = self.block_list_state.selected() {
                        if idx < self.blocks.len() {
                            self.push_nav();
                            let block_num = self.blocks[idx].block_num;
                            self.browse_block(block_num);
                            self.active_pane = Pane::BlockDetail;
                            self.mode = Mode::Browsing;
                        }
                    }
                }
                Pane::BlockDetail => {
                    if let Some(idx) = self.tx_list_state.selected() {
                        if idx < self.selected_block_txs.len() {
                            self.push_nav();
                            self.detail_scroll = 0;
                            self.active_pane = Pane::TxDetail;
                        }
                    }
                    // If no txs selected, try notes
                    if self.active_pane == Pane::BlockDetail
                        && self.selected_block_txs.is_empty()
                        && !self.selected_block_notes.is_empty()
                    {
                        self.push_nav();
                        self.note_list_state = ListState::default();
                        self.note_list_state.select(Some(0));
                        self.detail_scroll = 0;
                        self.active_pane = Pane::NoteDetail;
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
                Pane::BlockList => {
                    self.enter_tailing();
                }
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
            Action::ToggleTailing => {
                if self.mode == Mode::Tailing {
                    self.mode = Mode::Browsing;
                } else {
                    self.enter_tailing();
                }
            }
            Action::NewBlockReceived {
                block,
                transactions,
                notes,
            } => {
                // Always insert the block into the list
                let selected_block_num = self.selected_block_num();
                self.insert_block(block, transactions, notes);

                if self.mode == Mode::Tailing {
                    // Auto-select newest block
                    self.block_list_state.select(Some(0));
                } else if let Some(bn) = selected_block_num {
                    // Preserve the current selection by block number
                    self.select_block_by_num(bn);
                }
            }
            Action::SyncError(msg) => {
                self.error = Some(msg);
                self.error_time = Some(std::time::Instant::now());
            }
            Action::ScrollUp => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
            }
            Action::ScrollDown => {
                self.detail_scroll = self.detail_scroll.saturating_add(1);
            }
            Action::HalfPageUp => match self.active_pane {
                Pane::BlockList => {
                    self.push_nav();
                    self.mode = Mode::Browsing;
                    let current = self.block_list_state.selected().unwrap_or(0);
                    let target = current.saturating_sub(HALF_PAGE);
                    self.block_list_state.select(Some(target));
                }
                Pane::BlockDetail => {
                    let current = self.tx_list_state.selected().unwrap_or(0);
                    let target = current.saturating_sub(HALF_PAGE);
                    self.tx_list_state.select(Some(target));
                }
                Pane::TxDetail | Pane::NoteDetail => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(HALF_PAGE as u16);
                }
            },
            Action::HalfPageDown => match self.active_pane {
                Pane::BlockList => {
                    if !self.blocks.is_empty() {
                        self.push_nav();
                        self.mode = Mode::Browsing;
                        let current = self.block_list_state.selected().unwrap_or(0);
                        let target = (current + HALF_PAGE).min(self.blocks.len() - 1);
                        self.block_list_state.select(Some(target));
                    }
                }
                Pane::BlockDetail => {
                    if !self.selected_block_txs.is_empty() {
                        let current = self.tx_list_state.selected().unwrap_or(0);
                        let target = (current + HALF_PAGE).min(self.selected_block_txs.len() - 1);
                        self.tx_list_state.select(Some(target));
                    }
                }
                Pane::TxDetail | Pane::NoteDetail => {
                    self.detail_scroll = self.detail_scroll.saturating_add(HALF_PAGE as u16);
                }
            },
            Action::GoToTop => match self.active_pane {
                Pane::BlockList => {
                    if !self.blocks.is_empty() {
                        self.push_nav();
                        self.mode = Mode::Browsing;
                        self.block_list_state.select(Some(0));
                    }
                }
                Pane::BlockDetail => {
                    if !self.selected_block_txs.is_empty() {
                        self.tx_list_state.select(Some(0));
                    }
                }
                Pane::TxDetail | Pane::NoteDetail => {
                    self.detail_scroll = 0;
                }
            },
            Action::GoToBottom => match self.active_pane {
                Pane::BlockList => {
                    if !self.blocks.is_empty() {
                        self.push_nav();
                        self.mode = Mode::Browsing;
                        self.block_list_state.select(Some(self.blocks.len() - 1));
                    }
                }
                Pane::BlockDetail => {
                    if !self.selected_block_txs.is_empty() {
                        self.tx_list_state
                            .select(Some(self.selected_block_txs.len() - 1));
                    }
                }
                Pane::TxDetail | Pane::NoteDetail => {
                    self.detail_scroll = u16::MAX / 2;
                }
            },
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
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
            detail_scroll: self.detail_scroll,
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
        }

        if entry.pane != Pane::BlockList {
            self.mode = Mode::Browsing;
        }
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
        if !self.selected_block_txs.is_empty() {
            self.tx_list_state.select(Some(0));
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

    /// Get the block number currently selected in the block list
    fn selected_block_num(&self) -> Option<u32> {
        self.block_list_state
            .selected()
            .and_then(|i| self.blocks.get(i))
            .map(|b| b.block_num)
    }

    /// Update block_list_state to point at the given block number
    fn select_block_by_num(&mut self, block_num: u32) {
        if let Some(idx) = self.blocks.iter().position(|b| b.block_num == block_num) {
            self.block_list_state.select(Some(idx));
        }
    }

    fn enter_tailing(&mut self) {
        self.mode = Mode::Tailing;
        self.active_pane = Pane::BlockList;
        self.browsing_block_num = None;
        if !self.blocks.is_empty() {
            self.block_list_state.select(Some(0));
        }
    }

    fn select_next_block(&mut self, n: usize) {
        if self.blocks.is_empty() {
            return;
        }
        self.mode = Mode::Browsing;
        let i = match self.block_list_state.selected() {
            Some(i) => (i + n).min(self.blocks.len() - 1),
            None => 0,
        };
        self.block_list_state.select(Some(i));
    }

    fn select_prev_block(&mut self, n: usize) {
        if self.blocks.is_empty() {
            return;
        }
        self.mode = Mode::Browsing;
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
}
