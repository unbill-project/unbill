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
        1 + state.settlement.len().max(1) // separator + transactions or "settled up"
    } else {
        0
    };
    let settlement_height = (settlement_lines as u16).min(inner.height / 3);

    let split =
        Layout::vertical([Constraint::Min(0), Constraint::Length(settlement_height)]).split(inner);

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
        let scroll_offset = if state.bill_cursor >= visible_height {
            state.bill_cursor - visible_height + 1
        } else {
            0
        };

        for (i, bill) in state.bills.iter().enumerate().skip(scroll_offset) {
            let row_idx = i - scroll_offset;
            if row_idx >= visible_height {
                break;
            }
            let row = Rect {
                x: list_area.x,
                y: list_area.y + row_idx as u16,
                width: list_area.width,
                height: 1,
            };

            let is_cursor = i == state.bill_cursor;
            let style = if is_cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            // Description truncated to 30 chars, amount right-aligned.
            let desc = if bill.description.len() > 30 {
                format!("{}…", &bill.description[..29])
            } else {
                bill.description.clone()
            };
            let amount_str = format!("${}", format_cents(bill.amount_cents));

            let cols = Layout::horizontal([
                Constraint::Min(0),
                Constraint::Length(amount_str.len() as u16 + 1),
            ])
            .split(row);

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
            if settlement_area.height > 1 {
                frame.render_widget(
                    Paragraph::new("  settled up").style(Style::default().fg(Color::DarkGray)),
                    Rect {
                        x: settlement_area.x,
                        y: settlement_area.y + 1,
                        width: settlement_area.width,
                        height: 1,
                    },
                );
            }
        } else {
            for (i, txn) in state.settlement.iter().enumerate() {
                let line_y = settlement_area.y + 1 + i as u16;
                if line_y >= settlement_area.y + settlement_area.height {
                    break;
                }
                let from_name = resolve_user_name(&txn.from_user_id, &state.users);
                let to_name = resolve_user_name(&txn.to_user_id, &state.users);
                let amount_str = format!("${}", format_cents(txn.amount_cents));
                frame.render_widget(
                    Paragraph::new(format!("  {} → {}  {}", from_name, to_name, amount_str))
                        .style(Style::default().fg(Color::DarkGray)),
                    Rect {
                        x: settlement_area.x,
                        y: line_y,
                        width: settlement_area.width,
                        height: 1,
                    },
                );
            }
        }
    }
}

fn resolve_user_name(
    user_id: &unbill_core::model::Ulid,
    users: &[unbill_core::model::User],
) -> String {
    users
        .iter()
        .find(|u| &u.user_id == user_id)
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| {
            let s = user_id.to_string();
            s[..8.min(s.len())].to_string()
        })
}
