use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .error_log
        .iter()
        .enumerate()
        .map(|(i, msg)| {
            let content = Span::styled(
                format!("[{}] {}", i + 1, msg),
                Style::default().fg(Color::Red),
            );
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Error Log (!:close j/k:navigate c:clear) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.error_list_state);
}
