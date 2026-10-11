use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use crate::app::AppState;
use crate::pane::Pane;

fn truncate_name(name: &str) -> &str {
    match name.char_indices().nth(12) {
        #[allow(clippy::string_slice, reason = "char_indices returns UTF-8 boundaries")]
        Some((pos, _)) => &name[..pos],
        None => name,
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focused_pane == Pane::Ledgers;
    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::bordered()
        .title("Ledgers")
        .border_style(border_style);

    if state.ledgers.is_empty() && state.archived_count == 0 {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new("no ledgers — press [a] to create one")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let mut items: Vec<ListItem> = state
        .ledgers
        .iter()
        .map(|l| {
            let user_part = match state.ledger_user_names.get(&l.ledger_id) {
                Some(names) if names.len() > 3 => {
                    let visible: Vec<&str> =
                        names.iter().take(3).map(|n| truncate_name(n)).collect();
                    #[allow(
                        clippy::arithmetic_side_effects,
                        reason = "This branch requires names.len() > 3"
                    )]
                    let remaining = names.len() - 3;
                    format!("{} +{remaining} more", visible.join(", "))
                }
                Some(names) if !names.is_empty() => {
                    let parts: Vec<&str> = names.iter().map(|n| truncate_name(n)).collect();
                    parts.join(", ")
                }
                _ => "No users".to_owned(),
            };
            ListItem::new(format!(
                "{}{} ({} · {})",
                if l.archived { "  " } else { "" },
                l.name,
                user_part,
                l.currency.code()
            ))
        })
        .collect();

    let active_count = state.ledgers.iter().filter(|meta| !meta.archived).count();
    if state.archived_count > 0 {
        items.insert(
            active_count,
            ListItem::new(format!(
                "{} Archived ({}) [A]",
                if state.show_archived { "▾" } else { "▸" },
                state.archived_count
            ))
            .style(Style::default().fg(Color::DarkGray)),
        );
    }
    let mut list_state = ListState::default();
    if state.ledger_cursor < state.ledgers.len() {
        let cursor = if state.archived_count > 0 && state.ledger_cursor >= active_count {
            state.ledger_cursor.saturating_add(1)
        } else {
            state.ledger_cursor
        };
        list_state.select(Some(cursor));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(list, area, &mut list_state);
}
