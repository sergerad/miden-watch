use chrono::DateTime;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .blocks
        .iter()
        .map(|b| {
            let ts = DateTime::from_timestamp(b.timestamp as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| format!("{}", b.timestamp));

            let line = Line::from(vec![
                Span::styled(
                    format!(" Block #{:<8}", b.block_num),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" | "),
                Span::styled(ts, Style::default().fg(Color::White)),
                Span::raw(" | "),
                Span::styled(
                    format!("{} txs", b.tx_count),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{} notes", b.note_count),
                    Style::default().fg(Color::Green),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" miden-watch - Blocks ")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Cyan),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.block_list_state);
}
