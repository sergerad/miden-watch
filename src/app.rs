use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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

pub struct App {
    pub mode: Mode,
    pub active_pane: Pane,
    pub blocks: Vec<BlockInfo>,
    pub block_list_state: ListState,
    pub block_transactions: HashMap<u32, Vec<TransactionInfo>>,
    pub block_notes: HashMap<u32, Vec<NoteInfo>>,
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
            selected_block_txs: Vec::new(),
            selected_block_notes: Vec::new(),
            tx_list_state: ListState::default(),
            note_list_state: ListState::default(),
            error: None,
            error_time: None,
            should_quit: false,
            action_tx,
            detail_scroll: 0,
        }
    }

    pub fn handle_event(&self, event: Event) -> Option<Action> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Tick => Some(Action::Tick),
            Event::Resize(_, _) => None,
        }
    }

    fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(Action::Quit);
        }

        match self.active_pane {
            Pane::BlockList => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
                KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => Some(Action::Enter),
                KeyCode::Char('t') => Some(Action::ToggleTailing),
                _ => None,
            },
            Pane::BlockDetail => match key.code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                    Some(Action::Back)
                }
                KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
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
            Action::Up => match self.active_pane {
                Pane::BlockList => self.select_prev_block(),
                Pane::BlockDetail => self.select_prev_tx(),
                _ => {}
            },
            Action::Down => match self.active_pane {
                Pane::BlockList => self.select_next_block(),
                Pane::BlockDetail => self.select_next_tx(),
                _ => {}
            },
            Action::Enter => match self.active_pane {
                Pane::BlockList => {
                    if let Some(idx) = self.block_list_state.selected() {
                        if idx < self.blocks.len() {
                            let block_num = self.blocks[idx].block_num;
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
                            self.active_pane = Pane::BlockDetail;
                            self.mode = Mode::Browsing;
                        }
                    }
                }
                Pane::BlockDetail => {
                    if let Some(idx) = self.tx_list_state.selected() {
                        if idx < self.selected_block_txs.len() {
                            self.detail_scroll = 0;
                            self.active_pane = Pane::TxDetail;
                        }
                    }
                    // If no txs selected, try notes
                    if self.active_pane == Pane::BlockDetail
                        && self.selected_block_txs.is_empty()
                        && !self.selected_block_notes.is_empty()
                    {
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
                    self.active_pane = Pane::BlockList;
                }
                Pane::TxDetail | Pane::NoteDetail => {
                    self.active_pane = Pane::BlockDetail;
                    self.detail_scroll = 0;
                }
                Pane::BlockList => {
                    // Resume tailing from block list
                    self.mode = Mode::Tailing;
                    if !self.blocks.is_empty() {
                        self.block_list_state.select(Some(0));
                    }
                }
            },
            Action::ToggleTailing => {
                if self.mode == Mode::Tailing {
                    self.mode = Mode::Browsing;
                } else {
                    self.mode = Mode::Tailing;
                    self.active_pane = Pane::BlockList;
                    if !self.blocks.is_empty() {
                        self.block_list_state.select(Some(0));
                    }
                }
            }
            Action::NewBlockReceived {
                block,
                transactions,
                notes,
            } => {
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

                // Auto-select first block when tailing
                if self.mode == Mode::Tailing {
                    self.block_list_state.select(Some(0));
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
        }
    }

    fn select_next_block(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        self.mode = Mode::Browsing;
        let i = match self.block_list_state.selected() {
            Some(i) => {
                if i >= self.blocks.len() - 1 {
                    self.blocks.len() - 1
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.block_list_state.select(Some(i));
    }

    fn select_prev_block(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        self.mode = Mode::Browsing;
        let i = match self.block_list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.block_list_state.select(Some(i));
    }

    fn select_next_tx(&mut self) {
        if self.selected_block_txs.is_empty() {
            return;
        }
        let i = match self.tx_list_state.selected() {
            Some(i) => {
                if i >= self.selected_block_txs.len() - 1 {
                    self.selected_block_txs.len() - 1
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.tx_list_state.select(Some(i));
    }

    fn select_prev_tx(&mut self) {
        if self.selected_block_txs.is_empty() {
            return;
        }
        let i = match self.tx_list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.tx_list_state.select(Some(i));
    }

    pub fn selected_block(&self) -> Option<&BlockInfo> {
        self.block_list_state
            .selected()
            .and_then(|i| self.blocks.get(i))
    }

    pub fn selected_tx(&self) -> Option<&TransactionInfo> {
        self.tx_list_state
            .selected()
            .and_then(|i| self.selected_block_txs.get(i))
    }
}
