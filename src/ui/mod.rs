mod block_detail;
mod block_list;
mod note_detail;
mod status_bar;
mod tx_detail;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::{App, Pane};

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(frame.area());

    match app.active_pane {
        Pane::BlockList => {
            block_list::render(frame, app, chunks[0]);
        }
        Pane::BlockDetail => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(chunks[0]);
            block_detail::render_block_info(frame, app, main_chunks[0]);
            block_detail::render_tx_and_notes(frame, app, main_chunks[1]);
        }
        Pane::TxDetail => {
            tx_detail::render(frame, app, chunks[0]);
        }
        Pane::NoteDetail => {
            note_detail::render(frame, app, chunks[0]);
        }
    }

    status_bar::render(frame, app, chunks[1]);
}
