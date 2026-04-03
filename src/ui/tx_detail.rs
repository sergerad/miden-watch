use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let tx = match app.selected_tx() {
        Some(t) => t.clone(),
        None => {
            let p = Paragraph::new("No transaction selected").block(
                Block::default()
                    .title(" Transaction Detail ")
                    .borders(Borders::ALL),
            );
            frame.render_widget(p, area);
            return;
        }
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                "  Transaction ID:     ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(&tx.tx_id, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Account ID:         ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(&tx.account_id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Block Number:       ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("#{}", tx.block_num),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Notes", Style::default().fg(Color::Yellow))),
        Line::from(vec![
            Span::styled(
                "  Input Notes:        ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{}", tx.input_note_count),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Output Notes:       ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{}", tx.output_note_count),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Transaction Detail ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));

    frame.render_widget(paragraph, area);
}
