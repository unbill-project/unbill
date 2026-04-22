use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Paragraph},
};
use unbill_core::model::{Ulid, User};

use crate::app::AppState;
use crate::pane::Pane;

// ---------------------------------------------------------------------------
// BillEditor types (pub — used by app.rs)
// ---------------------------------------------------------------------------

pub struct ParticipantRow {
    pub user: User,
    pub selected: bool,
    pub weight: u32,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum EditorSection {
    Description,
    Amount,
    Payers,
    Payees,
}

pub struct BillEditor {
    pub ledger_id: String,
    pub prev_id: Option<Ulid>,
    pub description: String,
    pub amount_str: String,
    pub payers: Vec<ParticipantRow>,
    pub payees: Vec<ParticipantRow>,
    pub payer_cursor: usize,
    pub payee_cursor: usize,
    pub section: EditorSection,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focused_pane == Pane::Detail;
    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::bordered().title("Detail").border_style(border_style);

    if let Some(editor) = &state.bill_editor {
        render_editor(frame, area, block, editor);
    } else {
        render_view(frame, area, block, state);
    }
}

fn render_view(frame: &mut Frame, area: Rect, block: Block, state: &AppState) {
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(bill) = state.bills.get(state.bill_cursor) {
        // Bill detail read-only view.
        let rows = Layout::vertical([
            Constraint::Length(1), // description
            Constraint::Length(1), // amount
            Constraint::Length(1), // payers label
            Constraint::Min(0),    // payers list
            Constraint::Length(1), // hint
        ])
        .split(inner);

        frame.render_widget(
            Paragraph::new(format!("Description: {}", bill.description)),
            rows[0],
        );

        let dollars = bill.amount_cents / 100;
        let cents = bill.amount_cents.abs() % 100;
        frame.render_widget(
            Paragraph::new(format!("Amount: ${}.{:02}", dollars, cents)),
            rows[1],
        );

        // Combine payers and payees into rows[2] and rows[3].
        // Use rows[2] as a label for payers, rows[3] for the actual content.
        frame.render_widget(
            Paragraph::new("Payers / Payees:").style(Style::default().fg(Color::DarkGray)),
            rows[2],
        );

        // Render payers + payees into available space.
        let available = rows[3].height as usize;
        let mut line_idx = 0usize;

        for share in &bill.payers {
            if line_idx >= available {
                break;
            }
            let name = resolve_user_name(&share.user_id, &state.users);
            let row = Rect {
                x: rows[3].x,
                y: rows[3].y + line_idx as u16,
                width: rows[3].width,
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(format!("  pays: {} (×{})", name, share.shares))
                    .style(Style::default().fg(Color::DarkGray)),
                row,
            );
            line_idx += 1;
        }
        for share in &bill.payees {
            if line_idx >= available {
                break;
            }
            let name = resolve_user_name(&share.user_id, &state.users);
            let row = Rect {
                x: rows[3].x,
                y: rows[3].y + line_idx as u16,
                width: rows[3].width,
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(format!("  owes: {} (×{})", name, share.shares))
                    .style(Style::default().fg(Color::DarkGray)),
                row,
            );
            line_idx += 1;
        }

        frame.render_widget(
            Paragraph::new("[e] amend  [a] new").style(Style::default().fg(Color::DarkGray)),
            rows[4],
        );
    } else {
        // No bill selected.
        frame.render_widget(
            Paragraph::new("no bill selected — press [a] to add one")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
    }
}

fn render_editor(frame: &mut Frame, area: Rect, block: Block, editor: &BillEditor) {
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let user_count = editor.payers.len().max(1);
    let rows = Layout::vertical([
        Constraint::Length(1),                 // description
        Constraint::Length(1),                 // amount
        Constraint::Length(1),                 // payers label
        Constraint::Length(user_count as u16), // payers list
        Constraint::Length(1),                 // payees label
        Constraint::Length(user_count as u16), // payees list
        Constraint::Length(1),                 // live preview / error
        Constraint::Length(1),                 // hint
    ])
    .split(inner);

    // Description row.
    let desc_label_style = if editor.section == EditorSection::Description {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let desc_value_style = if editor.section == EditorSection::Description {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    let desc_cols = Layout::horizontal([Constraint::Length(14), Constraint::Min(0)]).split(rows[0]);
    frame.render_widget(
        Paragraph::new("Description: ").style(desc_label_style),
        desc_cols[0],
    );
    frame.render_widget(
        Paragraph::new(format!("{}_", editor.description)).style(desc_value_style),
        desc_cols[1],
    );

    // Amount row.
    let amt_label_style = if editor.section == EditorSection::Amount {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let amt_value_style = if editor.section == EditorSection::Amount {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    let amt_cols = Layout::horizontal([Constraint::Length(14), Constraint::Min(0)]).split(rows[1]);
    frame.render_widget(
        Paragraph::new("Amount:       ").style(amt_label_style),
        amt_cols[0],
    );
    frame.render_widget(
        Paragraph::new(format!("{}_", editor.amount_str)).style(amt_value_style),
        amt_cols[1],
    );

    // Payers label.
    let payers_label_style = if editor.section == EditorSection::Payers {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    frame.render_widget(Paragraph::new("Payers:").style(payers_label_style), rows[2]);

    // Payers list.
    for (i, row_data) in editor.payers.iter().enumerate() {
        if rows[3].height == 0 || i >= rows[3].height as usize {
            break;
        }
        let row = Rect {
            x: rows[3].x,
            y: rows[3].y + i as u16,
            width: rows[3].width,
            height: 1,
        };
        let is_cursor = editor.section == EditorSection::Payers && i == editor.payer_cursor;
        let style = if is_cursor {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let check = if row_data.selected { "x" } else { " " };
        frame.render_widget(
            Paragraph::new(format!(
                "[{}] {}  ×{}",
                check, row_data.user.display_name, row_data.weight
            ))
            .style(style),
            row,
        );
    }

    // Payees label.
    let payees_label_style = if editor.section == EditorSection::Payees {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    frame.render_widget(Paragraph::new("Payees:").style(payees_label_style), rows[4]);

    // Payees list.
    for (i, row_data) in editor.payees.iter().enumerate() {
        if rows[5].height == 0 || i >= rows[5].height as usize {
            break;
        }
        let row = Rect {
            x: rows[5].x,
            y: rows[5].y + i as u16,
            width: rows[5].width,
            height: 1,
        };
        let is_cursor = editor.section == EditorSection::Payees && i == editor.payee_cursor;
        let style = if is_cursor {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let check = if row_data.selected { "x" } else { " " };
        frame.render_widget(
            Paragraph::new(format!(
                "[{}] {}  ×{}",
                check, row_data.user.display_name, row_data.weight
            ))
            .style(style),
            row,
        );
    }

    // Live preview or error.
    if let Some(err) = &editor.error {
        frame.render_widget(
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
            rows[6],
        );
    } else {
        // Compute preview: parse amount and show per-payee split.
        let preview = build_preview(editor);
        frame.render_widget(
            Paragraph::new(preview).style(Style::default().fg(Color::DarkGray)),
            rows[6],
        );
    }

    // Hint.
    frame.render_widget(
        Paragraph::new(
            "[Tab] next  [j/k] move  [Space] toggle  [0-9] weight  [Enter] confirm  [Esc] cancel",
        )
        .style(Style::default().fg(Color::DarkGray)),
        rows[7],
    );
}

fn build_preview(editor: &BillEditor) -> String {
    let amount_cents = match parse_amount_cents(&editor.amount_str) {
        Some(v) if v >= 0 => v,
        _ => return String::new(),
    };
    let selected_payees: Vec<&ParticipantRow> =
        editor.payees.iter().filter(|r| r.selected).collect();
    if selected_payees.is_empty() {
        return String::new();
    }
    let total_weight: u32 = selected_payees.iter().map(|r| r.weight).sum();
    if total_weight == 0 {
        return String::new();
    }
    let parts: Vec<String> = selected_payees
        .iter()
        .map(|r| {
            let share = (amount_cents * r.weight as i64) / total_weight as i64;
            format!(
                "{}: ${}.{:02}",
                r.user.display_name,
                share / 100,
                share.abs() % 100
            )
        })
        .collect();
    parts.join("  ")
}

fn resolve_user_name(user_id: &Ulid, users: &[User]) -> String {
    users
        .iter()
        .find(|u| &u.user_id == user_id)
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| {
            let s = user_id.to_string();
            s[..8.min(s.len())].to_string()
        })
}

fn parse_amount_cents(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some((whole, frac)) = s.split_once('.') {
        let whole: i64 = whole.parse().ok()?;
        let frac = match frac.len() {
            0 => 0i64,
            1 => frac.parse::<i64>().ok()? * 10,
            _ => frac[..2].parse::<i64>().ok()?,
        };
        Some(whole * 100 + frac)
    } else {
        let whole: i64 = s.parse().ok()?;
        Some(whole * 100)
    }
}
