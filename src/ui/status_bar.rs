use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Mode, Pane};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
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

    let block_info = if let Some(block) = app.blocks.first() {
        Span::styled(
            format!(" Block #{} ", block.block_num),
            Style::default().fg(Color::Cyan),
        )
    } else {
        Span::styled(
            " Waiting for blocks... ",
            Style::default().fg(Color::DarkGray),
        )
    };

    let blocks_count = Span::styled(
        format!(" {} blocks ", app.blocks.len()),
        Style::default().fg(Color::White),
    );

    let nav_help = match app.active_pane {
        Pane::BlockList => Span::styled(
            " q:quit  j/k:navigate  Enter:detail  t:tail ",
            Style::default().fg(Color::DarkGray),
        ),
        Pane::BlockDetail => Span::styled(
            " q:quit  j/k:navigate  Enter:tx detail  Esc:back  t:tail ",
            Style::default().fg(Color::DarkGray),
        ),
        Pane::TxDetail | Pane::NoteDetail => Span::styled(
            " q:quit  j/k:scroll  Esc:back  t:tail ",
            Style::default().fg(Color::DarkGray),
        ),
    };

    let error_span = if let Some(ref err) = app.error {
        Span::styled(format!(" ERR: {} ", err), Style::default().fg(Color::Red))
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        mode_span,
        Span::raw(" | "),
        block_info,
        Span::raw(" | "),
        blocks_count,
        Span::raw(" | "),
        nav_help,
        error_span,
    ]);

    let paragraph = Paragraph::new(line).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}
