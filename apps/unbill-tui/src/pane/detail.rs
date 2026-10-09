use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Paragraph},
};
use unbill_console::error::Result;
use unbill_console::model::{BillId, LedgerId, Share, User, UserId};
use unbill_console::service::UnbillConsole;

use crate::app::{AppState, parse_amount_cents};
use crate::pane::Pane;

// ---------------------------------------------------------------------------
// BillEditor types (pub — used by app.rs)
// ---------------------------------------------------------------------------

// sirno:witness:unbill-tui:begin
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
    pub ledger_id: LedgerId,
    pub prev_id: Option<BillId>,
    pub bill_id: BillId,
    pub description: String,
    pub amount_str: String,
    pub payers: Vec<ParticipantRow>,
    pub payees: Vec<ParticipantRow>,
    pub payer_cursor: usize,
    pub payee_cursor: usize,
    pub show_archived: bool,
    pub section: EditorSection,
    pub error: Option<String>,
}
// sirno:witness:unbill-tui:end

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, svc: &UnbillConsole) {
    let focused = state.focused_pane == Pane::Detail;
    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::bordered().title("Detail").border_style(border_style);

    if let Some(editor) = &state.bill_editor {
        render_editor(frame, area, block, editor, svc);
    } else {
        render_view(frame, area, block, state, svc);
    }
}

fn render_view(frame: &mut Frame, area: Rect, block: Block, state: &AppState, svc: &UnbillConsole) {
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
        .areas::<5>(inner);

        frame.render_widget(
            Paragraph::new(format!("Description: {}", bill.description)),
            rows[0],
        );

        frame.render_widget(
            Paragraph::new("[e] amend  [a] new").style(Style::default().fg(Color::DarkGray)),
            rows[4],
        );

        // sirno:witness:unbill-tui:begin
        let split = match svc.calculate_bill_split(
            &bill.payers,
            &bill.payees,
            bill.amount_cents,
            bill.id,
        ) {
            Ok(split) => split,
            Err(error) => {
                if let Some(area) = rows.get(3).copied() {
                    frame.render_widget(
                        Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red)),
                        area,
                    );
                }
                return;
            }
        };
        // sirno:witness:unbill-tui:end

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
        let mut available_rows = rows[3].rows();

        for ((user_id, cents), row) in split.payer_amounts.iter().zip(&mut available_rows) {
            let name = resolve_user_name(user_id, &state.users);
            frame.render_widget(
                Paragraph::new(format!(
                    "  pays: {}  ${}.{:02}",
                    name,
                    cents / 100,
                    cents.abs() % 100
                ))
                .style(Style::default().fg(Color::DarkGray)),
                row,
            );
        }
        for ((user_id, cents), row) in split.payee_amounts.iter().zip(available_rows) {
            let name = resolve_user_name(user_id, &state.users);
            frame.render_widget(
                Paragraph::new(format!(
                    "  owes: {}  ${}.{:02}",
                    name,
                    cents / 100,
                    cents.abs() % 100
                ))
                .style(Style::default().fg(Color::DarkGray)),
                row,
            );
        }
    } else {
        // No bill selected.
        frame.render_widget(
            Paragraph::new("no bill selected — press [a] to add one")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
    }
}

fn render_editor(
    frame: &mut Frame,
    area: Rect,
    block: Block,
    editor: &BillEditor,
    svc: &UnbillConsole,
) {
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
    .areas::<8>(inner);

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
    let desc_cols =
        Layout::horizontal([Constraint::Length(14), Constraint::Min(0)]).areas::<2>(rows[0]);
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
    let amt_cols =
        Layout::horizontal([Constraint::Length(14), Constraint::Min(0)]).areas::<2>(rows[1]);
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
    render_participants(
        frame,
        rows[3],
        &editor.payers,
        editor.payer_cursor,
        editor.section == EditorSection::Payers,
        editor.show_archived,
    );

    // Payees label.
    let payees_label_style = if editor.section == EditorSection::Payees {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    frame.render_widget(Paragraph::new("Payees:").style(payees_label_style), rows[4]);

    // Payees list.
    render_participants(
        frame,
        rows[5],
        &editor.payees,
        editor.payee_cursor,
        editor.section == EditorSection::Payees,
        editor.show_archived,
    );

    // Live preview or error.
    if let Some(err) = &editor.error {
        frame.render_widget(
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
            rows[6],
        );
    } else {
        // Compute preview: parse amount and show per-payee split.
        let (preview, color) = match build_preview(editor, svc) {
            Ok(preview) => (preview, Color::DarkGray),
            Err(error) => (error.to_string(), Color::Red),
        };
        frame.render_widget(
            Paragraph::new(preview).style(Style::default().fg(color)),
            rows[6],
        );
    }

    // Hint.
    frame.render_widget(
        Paragraph::new(
            "[Tab] next  [j/k] move  [Space] toggle  [a] archived  [0-9] weight  [Enter] confirm  [Esc] cancel",
        )
        .style(Style::default().fg(Color::DarkGray)),
        rows[7],
    );
}

fn render_participants(
    frame: &mut Frame,
    area: Rect,
    participants: &[ParticipantRow],
    cursor: usize,
    focused: bool,
    expanded: bool,
) {
    let active = participants.iter().filter(|row| !row.user.archived).count();
    let archived = participants.len().saturating_sub(active);
    let mut lines = Vec::new();
    for (i, row) in participants.iter().enumerate() {
        if i == active && archived > 0 {
            lines.push(ratatui::text::Line::raw(format!(
                "{} Archived ({archived}) [a]",
                if expanded { "v" } else { ">" }
            )));
        }
        if row.user.archived && !expanded {
            continue;
        }
        let text = format!(
            "{}[{}] {} ×{}",
            if row.user.archived { "  " } else { "" },
            if row.selected { "x" } else { " " },
            row.user.display_name,
            row.weight
        );
        let style = if focused && cursor == i {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(ratatui::text::Line::styled(text, style));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

// sirno:witness:unbill-tui:begin
fn build_preview(editor: &BillEditor, svc: &UnbillConsole) -> Result<String> {
    let amount_cents = match parse_amount_cents(&editor.amount_str) {
        Some(v) if v >= 0 => v,
        _ => return Ok(String::new()),
    };
    let payer_shares: Vec<Share> = editor
        .payers
        .iter()
        .filter(|r| r.selected)
        .map(|r| Share {
            user_id: r.user.user_id,
            shares: r.weight,
        })
        .collect();
    let payee_shares: Vec<Share> = editor
        .payees
        .iter()
        .filter(|r| r.selected)
        .map(|r| Share {
            user_id: r.user.user_id,
            shares: r.weight,
        })
        .collect();
    if payee_shares.is_empty() {
        return Ok(String::new());
    }
    let split =
        svc.calculate_bill_split(&payer_shares, &payee_shares, amount_cents, editor.bill_id)?;
    let parts: Vec<String> = split
        .payee_amounts
        .iter()
        .map(|(uid, cents)| {
            let name = editor
                .payees
                .iter()
                .find(|r| r.user.user_id == *uid)
                .map(|r| r.user.display_name.as_str())
                .unwrap_or("?");
            format!("{}: ${}.{:02}", name, cents / 100, cents.abs() % 100)
        })
        .collect();
    Ok(parts.join("  "))
}
// sirno:witness:unbill-tui:end

fn resolve_user_name(user_id: &UserId, users: &[User]) -> String {
    users
        .iter()
        .find(|u| u.user_id == *user_id)
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| user_id.to_string().chars().take(8).collect())
}
