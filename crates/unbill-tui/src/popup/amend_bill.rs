use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};
use unbill_core::model::{Bill, NewBill, Share, User};

use super::{PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field};
use super::add_bill::{format_cents, parse_amount_cents};

#[derive(PartialEq, Eq)]
enum Section {
    Description,
    Amount,
    Payer,
    Payees,
}

pub struct AmendBillPopup {
    ledger_id: String,
    original_bill_id: unbill_core::model::Ulid,
    users: Vec<User>,
    description: TextInput,
    amount: TextInput,
    payer_cursor: usize,
    payee_cursor: usize,
    payee_selected: Vec<bool>,
    section: Section,
    error: Option<String>,
}

impl AmendBillPopup {
    pub fn new(ledger_id: String, bill: &Bill, users: Vec<User>) -> Self {
        // Find payer index
        let payer_user_id = bill.payers.first().map(|s| s.user_id);
        let payer_cursor = payer_user_id
            .and_then(|pid| users.iter().position(|u| u.user_id == pid))
            .unwrap_or(0);

        // Pre-select payees
        let payee_ids: std::collections::HashSet<_> =
            bill.payees.iter().map(|s| s.user_id).collect();
        let payee_selected: Vec<bool> = users.iter().map(|u| payee_ids.contains(&u.user_id)).collect();

        Self {
            ledger_id,
            original_bill_id: bill.id,
            users,
            description: TextInput::with_value("Description", bill.description.clone()),
            amount: TextInput::with_value("Amount", format_cents(bill.amount_cents)),
            payer_cursor,
            payee_cursor: 0,
            payee_selected,
            section: Section::Description,
            error: None,
        }
    }

    fn advance_section(&mut self) {
        self.section = match self.section {
            Section::Description => Section::Amount,
            Section::Amount => Section::Payer,
            Section::Payer => Section::Payees,
            Section::Payees => Section::Description,
        };
    }

    fn retreat_section(&mut self) {
        self.section = match self.section {
            Section::Description => Section::Payees,
            Section::Amount => Section::Description,
            Section::Payer => Section::Amount,
            Section::Payees => Section::Payer,
        };
    }

    fn try_confirm(&mut self) -> PopupOutcome {
        let description = self.description.value.trim().to_string();
        if description.is_empty() {
            self.error = Some("Description must not be empty".to_string());
            return PopupOutcome::Pending;
        }

        let amount_str = self.amount.value.trim();
        let amount_cents = match parse_amount_cents(amount_str) {
            Some(v) if v > 0 => v,
            _ => {
                self.error = Some("Enter a valid positive amount (e.g. 12.50)".to_string());
                return PopupOutcome::Pending;
            }
        };

        if self.users.is_empty() {
            self.error = Some("No users in ledger".to_string());
            return PopupOutcome::Pending;
        }

        let payer_user = &self.users[self.payer_cursor];
        let payers = vec![Share { user_id: payer_user.user_id, shares: 1 }];

        let payees: Vec<Share> = self
            .users
            .iter()
            .zip(self.payee_selected.iter())
            .filter(|(_, sel)| **sel)
            .map(|(u, _)| Share { user_id: u.user_id, shares: 1 })
            .collect();

        if payees.is_empty() {
            self.error = Some("Select at least one payee".to_string());
            return PopupOutcome::Pending;
        }

        self.error = None;
        PopupOutcome::Action(PopupAction::AddBill {
            ledger_id: self.ledger_id.clone(),
            bill: NewBill {
                amount_cents,
                description,
                payers,
                payees,
                prev: vec![self.original_bill_id],
            },
        })
    }
}

impl PopupView for AmendBillPopup {
    fn title(&self) -> &str {
        "Amend Bill"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let user_count = self.users.len().max(1);
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(user_count as u16),
            Constraint::Length(1),
            Constraint::Length(user_count as u16),
            Constraint::Length(1),
        ])
        .split(inner);

        render_text_field(frame, rows[0], &self.description, self.section == Section::Description);
        render_text_field(frame, rows[1], &self.amount, self.section == Section::Amount);

        let payer_label_style = if self.section == Section::Payer {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(Paragraph::new("Payer:").style(payer_label_style), rows[2]);

        for (i, user) in self.users.iter().enumerate() {
            if rows[3].height == 0 || i >= rows[3].height as usize {
                break;
            }
            let row = Rect { x: rows[3].x, y: rows[3].y + i as u16, width: rows[3].width, height: 1 };
            let is_cursor = self.section == Section::Payer && i == self.payer_cursor;
            let style = if is_cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let marker = if i == self.payer_cursor { ">" } else { " " };
            frame.render_widget(
                Paragraph::new(format!("{} {}", marker, user.display_name)).style(style),
                row,
            );
        }

        let payees_label_style = if self.section == Section::Payees {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(Paragraph::new("Payees:").style(payees_label_style), rows[4]);

        for (i, (user, &selected)) in
            self.users.iter().zip(self.payee_selected.iter()).enumerate()
        {
            if rows[5].height == 0 || i >= rows[5].height as usize {
                break;
            }
            let row = Rect { x: rows[5].x, y: rows[5].y + i as u16, width: rows[5].width, height: 1 };
            let is_cursor = self.section == Section::Payees && i == self.payee_cursor;
            let style = if is_cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let check = if selected { "x" } else { " " };
            frame.render_widget(
                Paragraph::new(format!("[{}] {}", check, user.display_name)).style(style),
                row,
            );
        }

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                rows[6],
            );
        } else {
            frame.render_widget(
                Paragraph::new("[Tab] next  [j/k] move  [Space] toggle  [Enter] confirm  [Esc] cancel")
                    .style(Style::default().fg(Color::DarkGray)),
                rows[6],
            );
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => return PopupOutcome::Cancelled,
            KeyCode::Tab => self.advance_section(),
            KeyCode::BackTab => self.retreat_section(),
            KeyCode::Enter => {
                if self.section == Section::Payees {
                    return self.try_confirm();
                } else {
                    self.advance_section();
                }
            }
            KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = c;
            }
            KeyCode::Char(c) => {
                self.error = None;
                match self.section {
                    Section::Description => self.description.push(c),
                    Section::Amount => self.amount.push(c),
                    Section::Payer => {
                        if c == 'j' {
                            if !self.users.is_empty() {
                                self.payer_cursor = (self.payer_cursor + 1).min(self.users.len() - 1);
                            }
                        } else if c == 'k' {
                            self.payer_cursor = self.payer_cursor.saturating_sub(1);
                        }
                    }
                    Section::Payees => {
                        if c == 'j' {
                            if !self.users.is_empty() {
                                self.payee_cursor = (self.payee_cursor + 1).min(self.users.len() - 1);
                            }
                        } else if c == 'k' {
                            self.payee_cursor = self.payee_cursor.saturating_sub(1);
                        } else if c == ' ' && !self.payee_selected.is_empty() {
                            self.payee_selected[self.payee_cursor] =
                                !self.payee_selected[self.payee_cursor];
                        }
                    }
                }
            }
            KeyCode::Down => match self.section {
                Section::Payer => {
                    if !self.users.is_empty() {
                        self.payer_cursor = (self.payer_cursor + 1).min(self.users.len() - 1);
                    }
                }
                Section::Payees => {
                    if !self.users.is_empty() {
                        self.payee_cursor = (self.payee_cursor + 1).min(self.users.len() - 1);
                    }
                }
                _ => {}
            },
            KeyCode::Up => match self.section {
                Section::Payer => {
                    self.payer_cursor = self.payer_cursor.saturating_sub(1);
                }
                Section::Payees => {
                    self.payee_cursor = self.payee_cursor.saturating_sub(1);
                }
                _ => {}
            },
            KeyCode::Backspace => {
                self.error = None;
                match self.section {
                    Section::Description => self.description.pop(),
                    Section::Amount => self.amount.pop(),
                    _ => {}
                }
            }
            _ => {}
        }
        PopupOutcome::Pending
    }
}
