use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Paragraph},
};

use crate::app::AppState;
use crate::pane::Pane;

fn format_cents(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents.abs() % 100)
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focused_pane == Pane::Bills;
    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::bordered().title("Bills").border_style(border_style);

    if state.ledgers.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new("select a ledger").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split: bill list on top, settlement section at bottom.
    // Settlement section: 1 separator + number of transactions (min 1 for "settled up").
    let settlement_lines = if state.current_ledger_id().is_some() {
        state.settlement.len().max(1).saturating_add(1) // separator + transactions or "settled up"
    } else {
        0
    };
    let settlement_height = settlement_lines.min(usize::from(inner.height / 3)) as u16;

    let split = Layout::vertical([Constraint::Min(0), Constraint::Length(settlement_height)])
        .areas::<2>(inner);

    let list_area = split[0];
    let settlement_area = split[1];

    // Render bill list.
    if state.bills.is_empty() {
        frame.render_widget(
            Paragraph::new("no bills — press [a] to add one")
                .style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
    } else {
        let visible_height = list_area.height as usize;

        // Simple scroll: keep cursor visible.
        let scroll_offset = state
            .bill_cursor
            .saturating_add(1)
            .saturating_sub(visible_height);

        for ((i, bill), row) in state
            .bills
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .zip(list_area.rows())
        {
            let is_cursor = i == state.bill_cursor;
            let style = if is_cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            // Description truncated to 30 chars, amount right-aligned.
            let desc = truncate_description(&bill.description);
            let amount_str = format!("${}", format_cents(bill.amount_cents));
            #[allow(
                clippy::arithmetic_side_effects,
                reason = "Formatting an i64 amount with a currency symbol is far shorter than u16::MAX"
            )]
            let amount_width = amount_str.len() as u16 + 1;
            let cols = Layout::horizontal([Constraint::Min(0), Constraint::Length(amount_width)])
                .areas::<2>(row);

            frame.render_widget(Paragraph::new(desc).style(style), cols[0]);
            frame.render_widget(
                Paragraph::new(amount_str)
                    .style(style)
                    .alignment(Alignment::Right),
                cols[1],
            );
        }
    }

    // Render settlement section (only if a ledger is selected).
    if state.current_ledger_id().is_some() && settlement_height > 0 {
        // Separator line.
        if settlement_area.height > 0 {
            frame.render_widget(
                Paragraph::new("─ Settlement ─").style(Style::default().fg(Color::DarkGray)),
                Rect {
                    x: settlement_area.x,
                    y: settlement_area.y,
                    width: settlement_area.width,
                    height: 1,
                },
            );
        }

        if state.settlement.is_empty() {
            if let Some(row) = settlement_area.rows().nth(1) {
                frame.render_widget(
                    Paragraph::new("  settled up").style(Style::default().fg(Color::DarkGray)),
                    row,
                );
            }
        } else {
            for (txn, row) in state.settlement.iter().zip(settlement_area.rows().skip(1)) {
                let from_name = resolve_user_name(&txn.from_user_id, &state.users);
                let to_name = resolve_user_name(&txn.to_user_id, &state.users);
                let amount_str = format!("${}", format_cents(txn.amount_cents));
                frame.render_widget(
                    Paragraph::new(format!("  {} → {}  {}", from_name, to_name, amount_str))
                        .style(Style::default().fg(Color::DarkGray)),
                    row,
                );
            }
        }
    }
}

fn resolve_user_name(
    user_id: &unbill_console::model::UserId,
    users: &[unbill_console::model::User],
) -> String {
    users
        .iter()
        .find(|u| u.user_id == *user_id)
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| user_id.to_string().chars().take(8).collect())
}

// sirno:witness:unbill-tui:begin
fn truncate_description(description: &str) -> String {
    if description.chars().nth(30).is_some() {
        format!("{}…", description.chars().take(29).collect::<String>())
    } else {
        description.to_owned()
    }
}
// sirno:witness:unbill-tui:end

#[cfg(test)]
mod tests {
    use super::truncate_description;

    #[test]
    fn description_truncation_preserves_unicode() {
        let short = "午餐🍜";
        assert_eq!(truncate_description(short), short);
        let exact = "🍜".repeat(30);
        assert_eq!(truncate_description(&exact), exact);
        let long = "🍜".repeat(31);
        assert_eq!(truncate_description(&long), format!("{}…", "🍜".repeat(29)));
    }
}
