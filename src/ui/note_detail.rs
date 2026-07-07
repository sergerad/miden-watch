use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let note = match app.selected_note() {
        Some(n) => n.clone(),
        None => {
            let p = ratatui::widgets::Paragraph::new("No note selected").block(
                Block::default()
                    .title(" Note Detail ")
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
        ("Note ID".to_string(), note.note_id.clone()),
        ("Block Number".to_string(), format!("#{}", note.block_num)),
        ("Note Index".to_string(), format!("{}", note.note_index)),
        (
            "Standard".to_string(),
            note.standard_type.clone().unwrap_or_else(|| "—".to_string()),
        ),
        ("Type".to_string(), note.note_type.clone()),
        (
            "Tag".to_string(),
            format!("{} (0x{:08x})", note.tag, note.tag),
        ),
        ("Sender".to_string(), note.sender.clone()),
    ];
    if let Some(target) = &note.target {
        rows.push(("Addressed To".to_string(), target.clone()));
    }

    // Store rows in app for copy support
    app.detail_rows = rows.clone();

    let items: Vec<ListItem> = rows
        .iter()
        .map(|(label, value)| {
            let value_color = if label == "Note ID" {
                Color::Green
            } else if label == "Sender" {
                Color::Cyan
            } else if label == "Addressed To" {
                Color::Magenta
            } else if label == "Type" || label == "Standard" {
                Color::Yellow
            } else {
                Color::White
            };

            let display_value = if label == "Note ID" || label == "Sender" || label == "Addressed To"
            {
                fmt_hash(value)
            } else {
                value.clone()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("  {:<16}", label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(display_value, Style::default().fg(value_color)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = format!(
        " Note {}... [y:copy e:hashes] ",
        &note.note_id[..16.min(note.note_id.len())]
    );

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Green),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.detail_row_state);
}
