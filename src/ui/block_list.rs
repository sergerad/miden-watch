use chrono::{DateTime, Utc};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, BlockFilter};

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

fn format_delta(newer_ts: u32, older_ts: u32) -> String {
    if newer_ts <= older_ts {
        return String::new();
    }
    let diff = newer_ts - older_ts;
    if diff < 60 {
        format!("+{}s", diff)
    } else if diff < 3600 {
        format!("+{}m{}s", diff / 60, diff % 60)
    } else {
        format!("+{}h{}m", diff / 3600, (diff % 3600) / 60)
    }
}

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let filtered = app.filtered_indices();

    // Determine if there's a flash-highlighted block (within 2 seconds)
    let flash_block_num = app.last_received_block.and_then(|(bn, instant)| {
        if instant.elapsed().as_secs() < 2 {
            Some(bn)
        } else {
            None
        }
    });

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(fi, &ri)| {
            let b = &app.blocks[ri];

            let ts = DateTime::from_timestamp(b.timestamp as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| format!("{}", b.timestamp));

            let relative = format_relative(b.timestamp);

            // Compute delta to the next (older) block in filtered view
            let delta = if fi + 1 < filtered.len() {
                let older = &app.blocks[filtered[fi + 1]];
                format_delta(b.timestamp, older.timestamp)
            } else {
                String::new()
            };

            let is_flash = flash_block_num == Some(b.block_num);

            let block_num_style = if is_flash {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else {
                Style::default().fg(Color::Cyan)
            };

            // Fixed-width columns: relative 8 chars, delta 7 chars
            let time_col = format!("({relative})");
            let delta_col = if delta.is_empty() {
                String::new()
            } else {
                delta
            };

            let mut spans = vec![
                Span::styled(format!(" Block #{:<8}", b.block_num), block_num_style),
                Span::raw(" | "),
                Span::styled(ts, Style::default().fg(Color::White)),
                Span::styled(
                    format!(" {:<10}", time_col),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:<7}", delta_col),
                    Style::default().fg(Color::DarkGray),
                ),
            ];

            spans.push(Span::raw("| "));
            spans.push(Span::styled(
                format!("{} txs", b.tx_count),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("{} notes", b.note_count),
                Style::default().fg(Color::Green),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = if app.filter == BlockFilter::All {
        format!(" miden-watch [{}] - Blocks ", app.network)
    } else {
        format!(
            " miden-watch [{}] - Blocks (filtered: {}) ",
            app.network, app.filter
        )
    };

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Cyan),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.block_list_state);
}
