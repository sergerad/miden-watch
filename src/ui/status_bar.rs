use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let head_span = if let Some(block) = app.blocks.first() {
        Span::styled(
            format!(" Head: #{} ", block.block_num),
            Style::default().fg(Color::Cyan),
        )
    } else {
        Span::styled(" Head: -- ", Style::default().fg(Color::DarkGray))
    };

    let help_hint = Span::styled(" ?:help ", Style::default().fg(Color::DarkGray));

    let mut top_spans = vec![head_span, Span::raw(" | "), help_hint];

    if let Some((current, target)) = app.sync_progress {
        top_spans.push(Span::raw("| "));
        if app.sync_done {
            top_spans.push(Span::styled(
                format!("✔ Synced {}/{}", current, target),
                Style::default().fg(Color::Green),
            ));
        } else {
            top_spans.push(Span::styled(
                format!("Syncing {}/{}", current, target),
                Style::default().fg(Color::Yellow),
            ));
        }
    }

    if !app.error_log.is_empty() {
        top_spans.push(Span::raw(" | "));
        top_spans.push(Span::styled(
            format!("⚠ {} error(s)", app.error_log.len()),
            Style::default().fg(Color::Red),
        ));
        top_spans.push(Span::styled(
            " !:view",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let paragraph = Paragraph::new(Line::from(top_spans))
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}
