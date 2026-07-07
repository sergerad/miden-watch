use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let id = match app.pending_account.clone() {
        Some(id) => id,
        None => {
            let p = ratatui::widgets::Paragraph::new("No account selected").block(
                Block::default()
                    .title(" Account Detail ")
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

    let account = app.selected_account().cloned();

    // Build (label, value) rows — value is what `y` copies.
    let mut rows: Vec<(String, String)> = vec![("Account ID".to_string(), id.clone())];

    match &account {
        None => {
            rows.push(("Status".to_string(), "loading…".to_string()));
        }
        Some(acct) => {
            rows.push(("Account Type".to_string(), acct.account_type.clone()));
            rows.push((
                "Public".to_string(),
                if acct.is_public { "yes" } else { "no" }.to_string(),
            ));

            match &acct.live_state {
                Some(ls) => {
                    rows.push(("Nonce".to_string(), ls.nonce.clone()));
                    rows.push(("Assets".to_string(), format!("{}", ls.num_assets)));
                    for asset in &ls.assets {
                        rows.push(("  •".to_string(), asset.clone()));
                    }
                    rows.push(("Storage".to_string(), ls.storage_commitment.clone()));
                }
                None => {
                    if let Some(err) = &acct.error {
                        rows.push(("Error".to_string(), err.clone()));
                    } else {
                        rows.push((
                            "State".to_string(),
                            "no public state (private account)".to_string(),
                        ));
                    }
                }
            }

            // Local history (from observed blocks).
            rows.push((String::new(), String::new()));
            rows.push((format!("Transactions ({})", acct.txs.len()), String::new()));
            for tx in &acct.txs {
                rows.push((format!("  tx #{}", tx.block_num), tx.tx_id.clone()));
            }
            rows.push((format!("Notes sent ({})", acct.sent_notes.len()), String::new()));
            for note in &acct.sent_notes {
                let kind = note
                    .standard_type
                    .clone()
                    .unwrap_or_else(|| note.note_type.clone());
                rows.push((format!("  {} #{}", kind, note.block_num), note.note_id.clone()));
            }
            rows.push((
                format!("Notes received ({})", acct.received_notes.len()),
                String::new(),
            ));
            for note in &acct.received_notes {
                let kind = note
                    .standard_type
                    .clone()
                    .unwrap_or_else(|| note.note_type.clone());
                rows.push((format!("  {} #{}", kind, note.block_num), note.note_id.clone()));
            }
        }
    }

    // Store rows for copy support.
    app.detail_rows = rows.clone();

    let items: Vec<ListItem> = rows
        .iter()
        .map(|(label, value)| {
            // Section headers have an empty value and a "(n)" count label.
            let is_header = value.is_empty() && label.ends_with(')');
            if is_header {
                return ListItem::new(Line::from(Span::styled(
                    format!(" {label}"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            if label.is_empty() && value.is_empty() {
                return ListItem::new(Line::from(Span::raw("")));
            }

            let value_color = match label.as_str() {
                "Account ID" => Color::Cyan,
                "Error" => Color::Red,
                "Public" => {
                    if value == "yes" {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }
                }
                _ if label.starts_with("  tx") => Color::Cyan,
                _ if label.starts_with("  ") && !value.is_empty() => Color::Green,
                _ => Color::White,
            };

            // Truncate anything that looks like a hex id/commitment.
            let display_value = if value.starts_with("0x") {
                fmt_hash(value)
            } else {
                value.clone()
            };

            let line = Line::from(vec![
                Span::styled(format!("  {:<14}", label), Style::default().fg(Color::DarkGray)),
                Span::styled(display_value, Style::default().fg(value_color)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = format!(
        " Account {}… [⏎:open y:copy e:hashes] ",
        &id[..16.min(id.len())]
    );

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Cyan),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.detail_row_state);
}
