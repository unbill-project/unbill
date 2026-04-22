// InvitePopup (two-tab) removed — invite is now in LedgerSettingsPopup.
// Only InviteResultPopup remains here.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};

use super::{PopupOutcome, PopupView, render_popup_base};

// ---------------------------------------------------------------------------
// InviteResultPopup — shows the generated URL
// ---------------------------------------------------------------------------

pub struct InviteResultPopup {
    title: &'static str,
    url: String,
}

impl InviteResultPopup {
    pub fn new(url: String) -> Self {
        Self {
            title: "Invite URL",
            url,
        }
    }

    pub fn with_title(title: &'static str, url: String) -> Self {
        Self { title, url }
    }
}

impl PopupView for InviteResultPopup {
    fn title(&self) -> &str {
        self.title
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Min(0),    // url
            Constraint::Length(1), // hint
        ])
        .split(inner);

        frame.render_widget(
            Paragraph::new(self.url.as_str()).wrap(ratatui::widgets::Wrap { trim: false }),
            rows[0],
        );
        frame.render_widget(
            Paragraph::new("[Esc] close").style(Style::default().fg(Color::DarkGray)),
            rows[1],
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            _ => PopupOutcome::Pending,
        }
    }
}
