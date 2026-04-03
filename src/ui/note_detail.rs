use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    // Show details for the first note or selected note
    let note = app.selected_block_notes.first().cloned();

    let note = match note {
        Some(n) => n,
        None => {
            let p = Paragraph::new("No note selected").block(
                Block::default()
                    .title(" Note Detail ")
                    .borders(Borders::ALL),
            );
            frame.render_widget(p, area);
            return;
        }
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  Note ID:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&note.note_id, Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Block Number: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("#{}", note.block_num),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Note Index:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", note.note_index),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Type:         ", Style::default().fg(Color::DarkGray)),
            Span::styled(&note.note_type, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("  Tag:          ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", note.tag), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Sender:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(&note.sender, Style::default().fg(Color::Cyan)),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Note Detail ")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));

    frame.render_widget(paragraph, area);
}
