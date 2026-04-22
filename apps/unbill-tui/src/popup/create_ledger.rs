use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};

use super::{
    PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field,
};

pub struct CreateLedgerPopup {
    name: TextInput,
    currency: TextInput,
    focused_field: usize,
    error: Option<String>,
}

impl CreateLedgerPopup {
    pub fn new() -> Self {
        Self {
            name: TextInput::new("Name"),
            currency: TextInput::new("Currency"),
            focused_field: 0,
            error: None,
        }
    }
}

impl PopupView for CreateLedgerPopup {
    fn title(&self) -> &str {
        "Create Ledger"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Length(1), // name field
            Constraint::Length(1), // currency field
            Constraint::Length(1), // spacer
            Constraint::Length(1), // error / hint
        ])
        .split(inner);

        render_text_field(frame, rows[0], &self.name, self.focused_field == 0);
        render_text_field(frame, rows[1], &self.currency, self.focused_field == 1);

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                rows[3],
            );
        } else {
            frame.render_widget(
                Paragraph::new("[Tab] next field  [Enter] confirm  [Esc] cancel")
                    .style(Style::default().fg(Color::DarkGray)),
                rows[3],
            );
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => return PopupOutcome::Cancelled,
            KeyCode::Tab => {
                self.focused_field = (self.focused_field + 1) % 2;
            }
            KeyCode::BackTab => {
                self.focused_field = if self.focused_field == 0 { 1 } else { 0 };
            }
            KeyCode::Down => {
                self.focused_field = (self.focused_field + 1) % 2;
            }
            KeyCode::Up => {
                self.focused_field = if self.focused_field == 0 { 1 } else { 0 };
            }
            KeyCode::Enter => {
                let name = self.name.value.trim().to_string();
                let currency = self.currency.value.trim().to_string();
                if name.is_empty() {
                    self.error = Some("Name must not be empty".to_string());
                    return PopupOutcome::Pending;
                }
                if currency.is_empty() {
                    self.error = Some("Currency must not be empty".to_string());
                    return PopupOutcome::Pending;
                }
                return PopupOutcome::Action(PopupAction::CreateLedger { name, currency });
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return PopupOutcome::Pending;
                }
                self.error = None;
                match self.focused_field {
                    0 => self.name.push(c),
                    _ => self.currency.push(c),
                }
            }
            KeyCode::Backspace => {
                self.error = None;
                match self.focused_field {
                    0 => self.name.pop(),
                    _ => self.currency.pop(),
                }
            }
            _ => {}
        }
        PopupOutcome::Pending
    }
}
