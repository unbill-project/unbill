use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::Paragraph,
};

use super::{PopupAction, PopupOutcome, PopupView, render_popup_base};

pub struct ConfirmPopup {
    pub message: String,
    /// The action to execute if the user selects Yes.
    pub action: PopupAction,
    /// `true` = Yes highlighted, `false` = No highlighted.
    pub selected: bool,
}

impl ConfirmPopup {
    pub fn new(message: String, action: PopupAction) -> Self {
        Self {
            message,
            action,
            selected: false,
        }
    }
}

impl PopupView for ConfirmPopup {
    fn title(&self) -> &str {
        "Confirm"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

        frame.render_widget(Paragraph::new(self.message.as_str()), rows[1]);

        let yes_style = if self.selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let no_style = if !self.selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        let cols = Layout::horizontal([
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(0),
        ])
        .split(rows[2]);

        frame.render_widget(Paragraph::new(" [Yes] ").style(yes_style), cols[0]);
        frame.render_widget(Paragraph::new(" [No]  ").style(no_style), cols[1]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            KeyCode::Char('j') | KeyCode::Char('l') | KeyCode::Down | KeyCode::Right => {
                self.selected = !self.selected;
                PopupOutcome::Pending
            }
            KeyCode::Char('k') | KeyCode::Char('h') | KeyCode::Up | KeyCode::Left => {
                self.selected = !self.selected;
                PopupOutcome::Pending
            }
            KeyCode::Enter => {
                if self.selected {
                    // We need to take the action out of self. Use a dummy replacement.
                    let action = std::mem::replace(
                        &mut self.action,
                        PopupAction::CreateLedger {
                            name: String::new(),
                            currency: String::new(),
                        },
                    );
                    PopupOutcome::Action(action)
                } else {
                    PopupOutcome::Cancelled
                }
            }
            _ => PopupOutcome::Pending,
        }
    }
}
