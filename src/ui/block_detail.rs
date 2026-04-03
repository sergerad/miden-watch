use chrono::DateTime;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::App;

pub fn render_block_info(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = match app.selected_block() {
        Some(b) => b,
        None => {
            let p = Paragraph::new("No block selected").block(
                Block::default()
                    .title(" Block Detail ")
                    .borders(Borders::ALL),
            );
            frame.render_widget(p, area);
            return;
        }
    };

    let ts = DateTime::from_timestamp(block.timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("{}", block.timestamp));

    let truncate = |s: &str| -> String {
        if s.len() > 16 {
            format!("{}...{}", &s[..8], &s[s.len() - 8..])
        } else {
            s.to_string()
        }
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  Block Number: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("#{}", block.block_num),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Timestamp:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(ts, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Version:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", block.version),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Commitments",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(vec![
            Span::styled("  Prev Block:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&block.prev_block_commitment),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Chain:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&block.chain_commitment),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Account Root: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&block.account_root),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Nullifier:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&block.nullifier_root),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Note Root:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&block.note_root),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tx Commit:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&block.tx_commitment),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tx Kernel:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                truncate(&block.tx_kernel_commitment),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    let title = format!(" Block #{} ", block.block_num);
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

pub fn render_tx_and_notes(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Transaction list
    let tx_items: Vec<ListItem> = app
        .selected_block_txs
        .iter()
        .map(|tx| {
            let id_short = if tx.tx_id.len() > 16 {
                format!("{}...", &tx.tx_id[..16])
            } else {
                tx.tx_id.clone()
            };
            let acct_short = if tx.account_id.len() > 12 {
                format!("{}...", &tx.account_id[..12])
            } else {
                tx.account_id.clone()
            };
            let line = Line::from(vec![
                Span::styled(format!(" {}", id_short), Style::default().fg(Color::Cyan)),
                Span::raw(" | "),
                Span::styled(acct_short, Style::default().fg(Color::Yellow)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let tx_list = List::new(tx_items)
        .block(
            Block::default()
                .title(format!(" Transactions ({}) ", app.selected_block_txs.len()))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Cyan),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(tx_list, chunks[0], &mut app.tx_list_state);

    // Note list
    let note_items: Vec<ListItem> = app
        .selected_block_notes
        .iter()
        .map(|note| {
            let id_short = if note.note_id.len() > 16 {
                format!("{}...", &note.note_id[..16])
            } else {
                note.note_id.clone()
            };
            let line = Line::from(vec![
                Span::styled(format!(" {}", id_short), Style::default().fg(Color::Green)),
                Span::raw(" | "),
                Span::styled(&note.note_type, Style::default().fg(Color::Yellow)),
                Span::raw(" | tag:"),
                Span::styled(format!("{}", note.tag), Style::default().fg(Color::White)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let note_list = List::new(note_items).block(
        Block::default()
            .title(format!(" Notes ({}) ", app.selected_block_notes.len()))
            .borders(Borders::ALL),
    );

    frame.render_widget(note_list, chunks[1]);
}
