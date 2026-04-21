use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Paragraph},
};

use crate::app::AppState;
use crate::pane::Pane;
use crate::popup::add_bill::format_cents;

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

    if state.bills.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new("no bills — press [a] to add one")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_height = inner.height as usize;

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
            x: inner.x,
            y: inner.y + row_idx as u16,
            width: inner.width,
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
            Paragraph::new(amount_str).style(style).alignment(Alignment::Right),
            cols[1],
        );
    }
}
