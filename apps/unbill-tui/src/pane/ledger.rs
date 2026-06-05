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

    if state.ledgers.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new("no ledgers — press [a] to create one")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = state
        .ledgers
        .iter()
        .map(|l| {
            let user_part = match state.ledger_user_names.get(&l.ledger_id) {
                Some(names) if names.len() > 3 => {
                    let visible: Vec<&str> = names[..3].iter().map(|n| truncate_name(n)).collect();
                    format!("{} +{} more", visible.join(", "), names.len() - 3)
                }
                Some(names) if !names.is_empty() => {
                    let parts: Vec<&str> = names.iter().map(|n| truncate_name(n)).collect();
                    parts.join(", ")
                }
                _ => "No users".to_owned(),
            };
            ListItem::new(format!(
                "{} ({} · {})",
                l.name,
                user_part,
                l.currency.code()
            ))
        })
        .collect();

    let mut list_state = ListState::default();
    if !state.ledgers.is_empty() {
        list_state.select(Some(state.ledger_cursor));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(list, area, &mut list_state);
}
