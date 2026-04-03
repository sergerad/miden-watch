mod block_detail;
mod block_list;
mod note_detail;
mod status_bar;
mod tx_detail;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, Pane};

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Main content
            Constraint::Length(4), // Status bar (2 rows + borders)
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

    if app.show_help {
        render_help_popup(frame);
    }
}

fn render_help_popup(frame: &mut Frame) {
    let area = frame.area();

    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 24u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(Color::Yellow);
    let desc_style = Style::default().fg(Color::White);

    let lines = vec![
        Line::from(Span::styled("Navigation", header_style)),
        Line::from(vec![
            Span::styled("  j/k, ↑/↓    ", key_style),
            Span::styled("Move up/down", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+u/d    ", key_style),
            Span::styled("Half-page up/down", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  gg          ", key_style),
            Span::styled("Go to top", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  G           ", key_style),
            Span::styled("Go to bottom", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("Actions", header_style)),
        Line::from(vec![
            Span::styled("  Enter, l, → ", key_style),
            Span::styled("Drill into selection", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Esc, h, ←   ", key_style),
            Span::styled("Go back", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  t           ", key_style),
            Span::styled("Toggle tailing mode", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+o      ", key_style),
            Span::styled("Jump back in history", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+i      ", key_style),
            Span::styled("Jump forward in history", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("General", header_style)),
        Line::from(vec![
            Span::styled("  ?           ", key_style),
            Span::styled("Toggle this help", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  q           ", key_style),
            Span::styled("Quit", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+c      ", key_style),
            Span::styled("Force quit", desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(popup, popup_area);
}
