use chrono::{DateTime, Utc};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, DetailFocus};

fn format_relative(ts: u32) -> String {
    let now = Utc::now().timestamp() as u32;
    if ts > now {
        return "just now".to_string();
    }
    let diff = now - ts;
    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

pub fn render_block_info(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = match app.selected_block() {
        Some(b) => b.clone(),
        None => {
            let p = ratatui::widgets::Paragraph::new("No block selected").block(
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

    let relative = format_relative(block.timestamp);

    // Compute block time delta from previous block
    let delta = {
        let block_num = block.block_num;
        let prev = app.blocks.iter().find(|b| b.block_num < block_num);
        prev.map(|p| {
            let diff = block.timestamp.saturating_sub(p.timestamp);
            if diff < 60 {
                format!("+{}s", diff)
            } else if diff < 3600 {
                format!("+{}m{}s", diff / 60, diff % 60)
            } else {
                format!("+{}h{}m", diff / 3600, (diff % 3600) / 60)
            }
        })
        .unwrap_or_default()
    };

    let expand = app.expand_hashes;
    let fmt_hash = |s: &str| -> String {
        if expand || s.len() <= 16 {
            s.to_string()
        } else {
            format!("{}...{}", &s[..8], &s[s.len() - 8..])
        }
    };

    // Build rows as (label, value) — value is what gets copied
    let mut rows: Vec<(String, String)> = vec![
        ("Block Number".to_string(), format!("#{}", block.block_num)),
        ("Timestamp".to_string(), format!("{} ({})", ts, relative)),
    ];

    if !delta.is_empty() {
        rows.push(("Block Time".to_string(), delta));
    }

    rows.push(("Version".to_string(), format!("{}", block.version)));
    rows.push((
        "Prev Block".to_string(),
        block.prev_block_commitment.clone(),
    ));
    rows.push(("Chain".to_string(), block.chain_commitment.clone()));
    rows.push(("Account Root".to_string(), block.account_root.clone()));
    rows.push(("Nullifier".to_string(), block.nullifier_root.clone()));
    rows.push(("Note Root".to_string(), block.note_root.clone()));
    rows.push(("Tx Commit".to_string(), block.tx_commitment.clone()));
    rows.push(("Tx Kernel".to_string(), block.tx_kernel_commitment.clone()));

    // Store rows in app for copy and navigation
    let is_block_focused = app.detail_focus == DetailFocus::Block;
    if is_block_focused {
        app.detail_rows = rows.clone();
    }

    let commitment_labels = [
        "Prev Block",
        "Chain",
        "Account Root",
        "Nullifier",
        "Note Root",
        "Tx Commit",
        "Tx Kernel",
    ];

    let items: Vec<ListItem> = rows
        .iter()
        .map(|(label, value)| {
            let is_commitment = commitment_labels.iter().any(|c| label == c);
            let value_color = if label == "Block Number" {
                Color::Cyan
            } else if is_commitment {
                Color::White
            } else {
                Color::White
            };

            let display_value = if is_commitment {
                fmt_hash(value)
            } else {
                value.clone()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("  {:<14}", label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(display_value, Style::default().fg(value_color)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let focus_hint = match app.detail_focus {
        DetailFocus::Block => " [Tab:txs]",
        _ => "",
    };
    let title = format!(" Block #{}{} ", block.block_num, focus_hint);

    let border_style = if is_block_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let mut list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_symbol(">> ");

    if is_block_focused {
        list = list.highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Cyan),
        );
    }

    if is_block_focused {
        frame.render_stateful_widget(list, area, &mut app.detail_row_state);
    } else {
        // Render without selection state when not focused
        frame.render_widget(list, area);
    }
}

pub fn render_tx_and_notes(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let focus = app.detail_focus;

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
            let mode_color = match tx.account_storage_mode.as_str() {
                "public" => Color::Green,
                "private" => Color::Red,
                "network" => Color::DarkGray,
                _ => Color::White,
            };
            let mode_label = match tx.account_storage_mode.as_str() {
                "public" => "pub",
                "private" => "priv",
                "network" => "net",
                _ => "?",
            };
            let line = Line::from(vec![
                Span::styled(format!(" {}", id_short), Style::default().fg(Color::Cyan)),
                Span::raw(" | "),
                Span::styled(acct_short, Style::default().fg(Color::Yellow)),
                Span::raw(" ["),
                Span::styled(mode_label, Style::default().fg(mode_color)),
                Span::raw("]"),
            ]);
            ListItem::new(line)
        })
        .collect();

    let tx_focused = focus == DetailFocus::Txs;
    let tx_border_style = if tx_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let tx_title = if tx_focused {
        format!(
            " Transactions ({}) [Tab:notes] ",
            app.selected_block_txs.len()
        )
    } else {
        format!(" Transactions ({}) ", app.selected_block_txs.len())
    };

    let mut tx_list = List::new(tx_items)
        .block(
            Block::default()
                .title(tx_title)
                .borders(Borders::ALL)
                .border_style(tx_border_style),
        )
        .highlight_symbol(">> ");

    if tx_focused {
        tx_list = tx_list.highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Cyan),
        );
    }

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

    let note_focused = focus == DetailFocus::Notes;
    let note_border_style = if note_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let note_title = if note_focused {
        format!(" Notes ({}) [Tab:block] ", app.selected_block_notes.len())
    } else {
        format!(" Notes ({}) ", app.selected_block_notes.len())
    };

    let mut note_list = List::new(note_items)
        .block(
            Block::default()
                .title(note_title)
                .borders(Borders::ALL)
                .border_style(note_border_style),
        )
        .highlight_symbol(">> ");

    if note_focused {
        note_list = note_list.highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Green),
        );
    }

    frame.render_stateful_widget(note_list, chunks[1], &mut app.note_list_state);
}
