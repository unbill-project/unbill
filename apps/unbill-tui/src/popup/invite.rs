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
    qr_text: String,
}

impl InviteResultPopup {
    // sirno:witness:unbill-tui:begin
    pub fn new(url: String) -> Self {
        let qr_text = unbill_console::qr::to_text(&url)
            .unwrap_or_else(|error| format!("QR code unavailable: {error}"));
        Self {
            title: "Invite URL",
            url,
            qr_text,
        }
    }
    // sirno:witness:unbill-tui:end
}

impl PopupView for InviteResultPopup {
    fn title(&self) -> &str {
        self.title
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let qr_lines = self.qr_text.lines().count() as u16;
        let rows = Layout::vertical([
            Constraint::Length(qr_lines), // QR code
            Constraint::Length(1),        // spacer
            Constraint::Min(0),           // url
            Constraint::Length(1),        // hint
        ])
        .areas::<4>(inner);

        frame.render_widget(Paragraph::new(self.qr_text.as_str()), rows[0]);
        frame.render_widget(
            Paragraph::new(self.url.as_str()).wrap(ratatui::widgets::Wrap { trim: false }),
            rows[2],
        );
        frame.render_widget(
            Paragraph::new("[Esc] close").style(Style::default().fg(Color::DarkGray)),
            rows[3],
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            _ => PopupOutcome::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_qr_keeps_the_invitation_url() {
        let url = "x".repeat(10_000);
        let popup = InviteResultPopup::new(url.clone());
        assert_eq!(popup.url, url);
        assert!(popup.qr_text.starts_with("QR code unavailable: "));
    }
}
