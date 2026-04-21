use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};
use unbill_core::model::{NewBill, Share, User};

use super::{PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field};

#[derive(PartialEq, Eq)]
enum AddBillSection {
    Description,
    Amount,
    Payer,
    Payees,
}

pub struct AddBillPopup {
    ledger_id: String,
    users: Vec<User>,
    description: TextInput,
    amount: TextInput,
    payer_cursor: usize,
    payee_cursor: usize,
    payee_selected: Vec<bool>,
    section: AddBillSection,
    error: Option<String>,
}

impl AddBillPopup {
    pub fn new(ledger_id: String, users: Vec<User>) -> Self {
        let payee_selected = vec![false; users.len()];
        Self {
            ledger_id,
            users,
            description: TextInput::new("Description"),
            amount: TextInput::new("Amount"),
            payer_cursor: 0,
            payee_cursor: 0,
            payee_selected,
            section: AddBillSection::Description,
            error: None,
        }
    }

    fn advance_section(&mut self) {
        self.section = match self.section {
            AddBillSection::Description => AddBillSection::Amount,
            AddBillSection::Amount => AddBillSection::Payer,
            AddBillSection::Payer => AddBillSection::Payees,
            AddBillSection::Payees => AddBillSection::Description,
        };
    }

    fn retreat_section(&mut self) {
        self.section = match self.section {
            AddBillSection::Description => AddBillSection::Payees,
            AddBillSection::Amount => AddBillSection::Description,
            AddBillSection::Payer => AddBillSection::Amount,
            AddBillSection::Payees => AddBillSection::Payer,
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
                prev: vec![],
            },
        })
    }
}

impl PopupView for AddBillPopup {
    fn title(&self) -> &str {
        "Add Bill"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        // Rows: description, amount, payer header, payer list, payees header, payees list, error
        let user_count = self.users.len().max(1);
        let rows = Layout::vertical([
            Constraint::Length(1),                          // description
            Constraint::Length(1),                          // amount
            Constraint::Length(1),                          // payer label
            Constraint::Length(user_count as u16),          // payer list
            Constraint::Length(1),                          // payees label
            Constraint::Length(user_count as u16),          // payees list
            Constraint::Length(1),                          // error/hint
        ])
        .split(inner);

        render_text_field(
            frame,
            rows[0],
            &self.description,
            self.section == AddBillSection::Description,
        );
        render_text_field(
            frame,
            rows[1],
            &self.amount,
            self.section == AddBillSection::Amount,
        );

        let payer_label_style = if self.section == AddBillSection::Payer {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(Paragraph::new("Payer:").style(payer_label_style), rows[2]);

        for (i, user) in self.users.iter().enumerate() {
            if rows[3].height == 0 || i >= rows[3].height as usize {
                break;
            }
            let row = Rect {
                x: rows[3].x,
                y: rows[3].y + i as u16,
                width: rows[3].width,
                height: 1,
            };
            let is_cursor = self.section == AddBillSection::Payer && i == self.payer_cursor;
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

        let payees_label_style = if self.section == AddBillSection::Payees {
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
            let row = Rect {
                x: rows[5].x,
                y: rows[5].y + i as u16,
                width: rows[5].width,
                height: 1,
            };
            let is_cursor = self.section == AddBillSection::Payees && i == self.payee_cursor;
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
                if self.section == AddBillSection::Payees {
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
                    AddBillSection::Description => self.description.push(c),
                    AddBillSection::Amount => self.amount.push(c),
                    AddBillSection::Payer => {
                        if c == 'j' {
                            if !self.users.is_empty() {
                                self.payer_cursor =
                                    (self.payer_cursor + 1).min(self.users.len() - 1);
                            }
                        } else if c == 'k' {
                            self.payer_cursor = self.payer_cursor.saturating_sub(1);
                        }
                    }
                    AddBillSection::Payees => {
                        if c == 'j' {
                            if !self.users.is_empty() {
                                self.payee_cursor =
                                    (self.payee_cursor + 1).min(self.users.len() - 1);
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
                AddBillSection::Payer => {
                    if !self.users.is_empty() {
                        self.payer_cursor = (self.payer_cursor + 1).min(self.users.len() - 1);
                    }
                }
                AddBillSection::Payees => {
                    if !self.users.is_empty() {
                        self.payee_cursor = (self.payee_cursor + 1).min(self.users.len() - 1);
                    }
                }
                _ => {}
            },
            KeyCode::Up => match self.section {
                AddBillSection::Payer => {
                    self.payer_cursor = self.payer_cursor.saturating_sub(1);
                }
                AddBillSection::Payees => {
                    self.payee_cursor = self.payee_cursor.saturating_sub(1);
                }
                _ => {}
            },
            KeyCode::Backspace => {
                self.error = None;
                match self.section {
                    AddBillSection::Description => self.description.pop(),
                    AddBillSection::Amount => self.amount.pop(),
                    _ => {}
                }
            }
            _ => {}
        }
        PopupOutcome::Pending
    }
}

/// Parse a decimal string like "12.50" or "12" into integer cents.
/// "12.50" → 1250, "12" → 1200.
pub fn parse_amount_cents(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some((whole, frac)) = s.split_once('.') {
        let whole: i64 = whole.parse().ok()?;
        let frac = match frac.len() {
            0 => 0i64,
            1 => frac.parse::<i64>().ok()? * 10,
            _ => frac[..2].parse::<i64>().ok()?,
        };
        Some(whole * 100 + frac)
    } else {
        let whole: i64 = s.parse().ok()?;
        Some(whole * 100)
    }
}

/// Format cents as a decimal string. 1250 → "12.50".
pub fn format_cents(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents.abs() % 100)
}
