use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let tx = match app.selected_tx() {
        Some(t) => t.clone(),
        None => {
            let p = ratatui::widgets::Paragraph::new("No transaction selected").block(
                Block::default()
                    .title(" Transaction Detail ")
                    .borders(Borders::ALL),
            );
            frame.render_widget(p, area);
            return;
        }
    };

    let expand = app.expand_hashes;
    let fmt_hash = |s: &str| -> String {
        if expand || s.len() <= 16 {
            s.to_string()
        } else {
            format!("{}...{}", &s[..8], &s[s.len() - 8..])
        }
    };

    // Build rows as (label, value) pairs — value is what gets copied
    let mut rows: Vec<(String, String)> = vec![
        ("Transaction ID".to_string(), tx.tx_id.clone()),
        ("Account ID".to_string(), tx.account_id.clone()),
        ("Block Number".to_string(), format!("#{}", tx.block_num)),
        (
            "Input Notes".to_string(),
            format!("{}", tx.input_note_count),
        ),
        (
            "Output Notes".to_string(),
            format!("{}", tx.output_note_count),
        ),
    ];

    // Add block notes as rows
    for note in &app.selected_block_notes {
        rows.push((
            format!("Note {}", &note.note_id[..12.min(note.note_id.len())]),
            note.note_id.clone(),
        ));
    }

    // Store rows in app for copy support
    app.detail_rows = rows.clone();

    let items: Vec<ListItem> = rows
        .iter()
        .map(|(label, value)| {
            let value_color = if label.starts_with("Transaction") {
                Color::Cyan
            } else if label.starts_with("Account") {
                Color::Yellow
            } else if label.starts_with("Note ") {
                Color::Green
            } else {
                Color::White
            };

            let display_value = if label.contains("ID") || label.starts_with("Note ") {
                fmt_hash(value)
            } else {
                value.clone()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("  {:<18}", label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(display_value, Style::default().fg(value_color)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = format!(
        " Transaction {}... [y:copy e:hashes] ",
        &tx.tx_id[..16.min(tx.tx_id.len())],
    );

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Cyan),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.detail_row_state);
}
