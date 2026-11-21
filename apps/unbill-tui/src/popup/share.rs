use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};
use unbill_console::model::LedgerId;

use super::{
    PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field,
};

pub struct SharePopup {
    ledger_id: LedgerId,
    account: TextInput,
}

impl SharePopup {
    pub fn new(ledger_id: LedgerId) -> Self {
        Self {
            ledger_id,
            account: TextInput::new("Account"),
        }
    }
}

impl PopupView for SharePopup {
    fn title(&self) -> &str {
        "Share ledger"
    }
    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());
        render_text_field(frame, inner, &self.account, true);
    }
    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            KeyCode::Enter if !self.account.value.trim().is_empty() => {
                PopupOutcome::Action(PopupAction::ShareLedger {
                    ledger_id: self.ledger_id,
                    account_name: self.account.value.trim().to_owned(),
                })
            }
            KeyCode::Char(c) => {
                self.account.push(c);
                PopupOutcome::Pending
            }
            KeyCode::Backspace => {
                self.account.pop();
                PopupOutcome::Pending
            }
            _ => PopupOutcome::Pending,
        }
    }
}
