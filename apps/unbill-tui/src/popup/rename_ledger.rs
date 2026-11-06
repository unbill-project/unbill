use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};
use unbill_console::model::LedgerId;

use super::{
    PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field,
};

pub struct RenameLedgerPopup {
    ledger_id: LedgerId,
    name: TextInput,
    invalid: bool,
}

impl RenameLedgerPopup {
    pub fn new(ledger_id: LedgerId, current_name: String) -> Self {
        let mut name = TextInput::new("Ledger name");
        name.value = current_name;
        Self {
            ledger_id,
            name,
            invalid: false,
        }
    }
}

impl PopupView for RenameLedgerPopup {
    fn title(&self) -> &str {
        "Rename Ledger"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());
        let [input, note, hint] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);
        render_text_field(frame, input, &self.name, true);
        let (text, color) = if self.invalid {
            (
                unbill_console::model::validation::LEDGER_NAME_ERROR,
                Color::Red,
            )
        } else {
            ("The name is shared when the ledger syncs.", Color::DarkGray)
        };
        frame.render_widget(Paragraph::new(text).style(Style::default().fg(color)), note);
        frame.render_widget(
            Paragraph::new("[Enter] save  [Ctrl+U] clear  [Esc] cancel"),
            hint,
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        self.invalid = false;
        match key.code {
            KeyCode::Esc => return PopupOutcome::Cancelled,
            KeyCode::Enter => {
                let name = self.name.value.trim();
                if unbill_console::model::validation::ledger_name(name).is_ok() {
                    return PopupOutcome::Action(PopupAction::RenameLedger {
                        ledger_id: self.ledger_id,
                        name: name.to_owned(),
                    });
                }
                self.invalid = true;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.name.value.clear();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => self.name.push(c),
            KeyCode::Backspace => self.name.pop(),
            _ => {}
        }
        PopupOutcome::Pending
    }
}
