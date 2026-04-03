use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Mode};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Draw the outer border
    let border = Block::default().borders(Borders::ALL);
    let inner = border.inner(area);
    frame.render_widget(border, area);

    // Split inner area into left (status) and right (error)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(38), Constraint::Min(1)])
        .split(inner);

    // Left column: mode | head | help
    let mode_span = match app.mode {
        Mode::Tailing => Span::styled(
            " TAILING ",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Mode::Browsing => Span::styled(
            " BROWSING ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ),
    };

    let head_span = if let Some(block) = app.blocks.first() {
        Span::styled(
            format!(" Head: #{} ", block.block_num),
            Style::default().fg(Color::Cyan),
        )
    } else {
        Span::styled(" Head: -- ", Style::default().fg(Color::DarkGray))
    };

    let help_hint = Span::styled(" ?:help ", Style::default().fg(Color::DarkGray));

    let left = Paragraph::new(Line::from(vec![
        mode_span,
        Span::raw(" | "),
        head_span,
        Span::raw(" | "),
        help_hint,
    ]));
    frame.render_widget(left, columns[0]);

    // Right column: error message (wraps across 2 lines if needed)
    if let Some(ref err) = app.error {
        let error = Paragraph::new(Span::styled(
            format!("ERR: {}", err),
            Style::default().fg(Color::Red),
        ))
        .wrap(Wrap { trim: false });
        frame.render_widget(error, columns[1]);
    }
}
