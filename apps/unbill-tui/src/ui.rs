use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};

use crate::app::{AppState, SyncStatus};
use crate::pane;
use crate::popup::centered_rect;

/// Top-level render function. Composes panes, popup, and status bar.
pub fn render(frame: &mut Frame, state: &AppState) {
    let full_area = frame.area();

    // Split vertically: main area + 1-line status bar.
    let vertical = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(full_area);

    let main_area = vertical[0];
    let status_area = vertical[1];

    // Split main horizontally: 20% ledger | 40% bills | 40% detail.
    let cols = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(40),
        Constraint::Percentage(40),
    ])
    .split(main_area);

    // Render panes.
    pane::ledger::render(frame, cols[0], state);
    pane::bills::render(frame, cols[1], state);
    pane::detail::render(frame, cols[2], state);

    // Status bar.
    render_status_bar(frame, status_area, state);

    // Popup overlay.
    if let Some(popup) = &state.popup {
        let popup_area = centered_rect(60, 70, full_area);
        popup.render(frame, popup_area);
    }
}

fn render_status_bar(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let hints = if state.popup.is_some() {
        "[Esc] close"
    } else {
        state.focused_pane.hints()
    };

    let sync_text = match &state.sync_status {
        SyncStatus::Idle => String::new(),
        SyncStatus::Syncing => "syncing…".to_string(),
        SyncStatus::Error(e) => format!("sync error: {}", e),
    };
    let sync_style = match &state.sync_status {
        SyncStatus::Error(_) => Style::default().fg(Color::Red),
        _ => Style::default().fg(Color::DarkGray),
    };

    let cols = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(sync_text.len().max(1) as u16),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(hints).style(Style::default().fg(Color::DarkGray)),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(sync_text)
            .style(sync_style)
            .alignment(Alignment::Right),
        cols[1],
    );
}
