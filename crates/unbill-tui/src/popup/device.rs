use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};

use super::{PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field};

pub struct DevicePopup {
    device_id: String,
    peer_input: TextInput,
    error: Option<String>,
}

impl DevicePopup {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            peer_input: TextInput::new("Peer NodeId"),
            error: None,
        }
    }
}

impl PopupView for DevicePopup {
    fn title(&self) -> &str {
        "Device Settings"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Length(1), // device id label
            Constraint::Length(1), // device id value
            Constraint::Length(1), // spacer
            Constraint::Length(1), // peer input
            Constraint::Length(1), // error / hint
        ])
        .split(inner);

        frame.render_widget(
            Paragraph::new("Device ID:").style(Style::default().fg(Color::DarkGray)),
            rows[0],
        );
        frame.render_widget(Paragraph::new(self.device_id.as_str()), rows[1]);

        render_text_field(frame, rows[3], &self.peer_input, true);

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                rows[4],
            );
        } else {
            frame.render_widget(
                Paragraph::new("[Enter] sync  [Esc] close")
                    .style(Style::default().fg(Color::DarkGray)),
                rows[4],
            );
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            KeyCode::Enter => {
                let peer_str = self.peer_input.value.trim().to_string();
                if peer_str.is_empty() {
                    self.error = Some("Enter a peer NodeId".to_string());
                    return PopupOutcome::Pending;
                }
                self.error = None;
                PopupOutcome::Action(PopupAction::SyncOnce { peer_node_id: peer_str })
            }
            KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = c;
                PopupOutcome::Pending
            }
            KeyCode::Char(c) => {
                self.error = None;
                self.peer_input.push(c);
                PopupOutcome::Pending
            }
            KeyCode::Backspace => {
                self.error = None;
                self.peer_input.pop();
                PopupOutcome::Pending
            }
            _ => PopupOutcome::Pending,
        }
    }
}
