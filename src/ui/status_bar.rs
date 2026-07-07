use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Pane};

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

    let latency_span = if let Some(ms) = app.latency_ms {
        let (symbol, color) = if ms < 100 {
            ("●", Color::Green)
        } else if ms < 500 {
            ("●", Color::Yellow)
        } else {
            ("●", Color::Red)
        };
        Span::styled(format!(" {} {}ms ", symbol, ms), Style::default().fg(color))
    } else {
        Span::styled(" ○ -- ", Style::default().fg(Color::DarkGray))
    };

    // Breadcrumb trail
    let breadcrumb = build_breadcrumb(app);

    let mut top_spans = vec![head_span, Span::raw(" | "), latency_span];

    if let Some((current, target)) = app.load_progress {
        top_spans.push(Span::raw("| "));
        top_spans.push(Span::styled(
            format!("Loading {}/{}", current, target),
            Style::default().fg(Color::DarkGray),
        ));
    } else if let Some((current, target)) = app.sync_progress {
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

    // Clipboard flash
    if let Some((ref msg, instant)) = app.clipboard_flash {
        if instant.elapsed().as_secs() < 2 {
            top_spans.push(Span::raw(" | "));
            top_spans.push(Span::styled(msg.clone(), Style::default().fg(Color::Green)));
        }
    }

    top_spans.push(Span::raw(" | "));
    top_spans.push(breadcrumb);
    top_spans.push(Span::raw(" | "));
    top_spans.push(help_hint);

    let paragraph = Paragraph::new(Line::from(top_spans))
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn build_breadcrumb(app: &App) -> Span<'static> {
    let trail = match app.active_pane {
        Pane::BlockList => "Blocks".to_string(),
        Pane::BlockDetail => {
            let bn = app
                .browsing_block_num
                .map(|n| format!("#{}", n))
                .unwrap_or_else(|| "?".to_string());
            format!("Blocks > Block {}", bn)
        }
        Pane::TxDetail => {
            let bn = app
                .browsing_block_num
                .map(|n| format!("#{}", n))
                .unwrap_or_else(|| "?".to_string());
            let tx_id = app
                .selected_tx()
                .map(|t| {
                    if t.tx_id.len() > 12 {
                        format!("{}...", &t.tx_id[..12])
                    } else {
                        t.tx_id.clone()
                    }
                })
                .unwrap_or_else(|| "?".to_string());
            format!("Blocks > {} > Tx {}", bn, tx_id)
        }
        Pane::NoteDetail => {
            let bn = app
                .browsing_block_num
                .map(|n| format!("#{}", n))
                .unwrap_or_else(|| "?".to_string());
            let note_id = app
                .selected_note()
                .map(|n| {
                    if n.note_id.len() > 12 {
                        format!("{}...", &n.note_id[..12])
                    } else {
                        n.note_id.clone()
                    }
                })
                .unwrap_or_else(|| "?".to_string());
            format!("Blocks > {} > Note {}", bn, note_id)
        }
        Pane::AccountDetail => {
            let id = app
                .pending_account
                .as_deref()
                .map(|id| {
                    if id.len() > 12 {
                        format!("{}...", &id[..12])
                    } else {
                        id.to_string()
                    }
                })
                .unwrap_or_else(|| "?".to_string());
            format!("Account {}", id)
        }
    };

    Span::styled(trail, Style::default().fg(Color::DarkGray))
}
