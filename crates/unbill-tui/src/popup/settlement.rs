use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};
use unbill_core::service::LocalUser;
use unbill_core::settlement::Transaction;

use super::{PopupAction, PopupOutcome, PopupView, render_popup_base};

// ---------------------------------------------------------------------------
// PickUserPopup
// ---------------------------------------------------------------------------

pub struct PickUserPopup {
    local_users: Vec<LocalUser>,
    cursor: usize,
}

impl PickUserPopup {
    pub fn new(local_users: Vec<LocalUser>) -> Self {
        Self { local_users, cursor: 0 }
    }
}

impl PopupView for PickUserPopup {
    fn title(&self) -> &str {
        "Settlement — Pick User"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Min(0),    // list
            Constraint::Length(1), // hint
        ])
        .split(inner);

        if self.local_users.is_empty() {
            frame.render_widget(
                Paragraph::new("no saved users — create one first")
                    .style(Style::default().fg(Color::DarkGray)),
                rows[0],
            );
        } else {
            for (i, user) in self.local_users.iter().enumerate() {
                if i >= rows[0].height as usize {
                    break;
                }
                let row = Rect {
                    x: rows[0].x,
                    y: rows[0].y + i as u16,
                    width: rows[0].width,
                    height: 1,
                };
                let is_cursor = i == self.cursor;
                let style = if is_cursor {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                let marker = if is_cursor { ">" } else { " " };
                frame.render_widget(
                    Paragraph::new(format!("{} {}", marker, user.display_name)).style(style),
                    row,
                );
            }
        }

        frame.render_widget(
            Paragraph::new("[j/k] move  [Enter] select  [Esc] cancel")
                .style(Style::default().fg(Color::DarkGray)),
            rows[1],
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.local_users.is_empty() {
                    self.cursor = (self.cursor + 1).min(self.local_users.len() - 1);
                }
                PopupOutcome::Pending
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                PopupOutcome::Pending
            }
            KeyCode::Enter => {
                if self.local_users.is_empty() {
                    PopupOutcome::Pending
                } else {
                    let user = &self.local_users[self.cursor];
                    PopupOutcome::Action(PopupAction::ShowSettlement {
                        user_id: user.user_id.to_string(),
                        display_name: user.display_name.clone(),
                    })
                }
            }
            _ => PopupOutcome::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// SettlementResultPopup
// ---------------------------------------------------------------------------

pub struct SettlementResultPopup {
    user_display_name: String,
    transactions: Vec<Transaction>,
    /// Map from user_id string to display_name for all known users.
    user_names: std::collections::HashMap<String, String>,
    scroll: usize,
}

impl SettlementResultPopup {
    pub fn new(
        user_display_name: String,
        transactions: Vec<Transaction>,
        user_names: std::collections::HashMap<String, String>,
    ) -> Self {
        Self { user_display_name, transactions, user_names, scroll: 0 }
    }

    fn resolve_name(&self, id: &unbill_core::model::Ulid) -> String {
        let key = id.to_string();
        self.user_names.get(&key).cloned().unwrap_or_else(|| key[..8.min(key.len())].to_string())
    }
}

impl PopupView for SettlementResultPopup {
    fn title(&self) -> &str {
        "Settlement"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Length(1), // header
            Constraint::Min(0),    // transactions
            Constraint::Length(1), // hint
        ])
        .split(inner);

        frame.render_widget(
            Paragraph::new(format!("Settlement for {}", self.user_display_name)),
            rows[0],
        );

        if self.transactions.is_empty() {
            frame.render_widget(
                Paragraph::new("all settled up!").style(Style::default().fg(Color::DarkGray)),
                rows[1],
            );
        } else {
            let visible_height = rows[1].height as usize;
            let start = self.scroll;
            for (i, txn) in self.transactions.iter().enumerate().skip(start) {
                let row_idx = i - start;
                if row_idx >= visible_height {
                    break;
                }
                let row = Rect {
                    x: rows[1].x,
                    y: rows[1].y + row_idx as u16,
                    width: rows[1].width,
                    height: 1,
                };
                let from = self.resolve_name(&txn.from_user_id);
                let to = self.resolve_name(&txn.to_user_id);
                let amount = format!(
                    "${}.{:02}",
                    txn.amount_cents / 100,
                    txn.amount_cents.abs() % 100
                );
                frame.render_widget(
                    Paragraph::new(format!("{} → {}   {}", from, to, amount)),
                    row,
                );
            }
        }

        frame.render_widget(
            Paragraph::new("[j/k] scroll  [Esc] close")
                .style(Style::default().fg(Color::DarkGray)),
            rows[2],
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            KeyCode::Char('j') | KeyCode::Down => {
                if self.scroll + 1 < self.transactions.len() {
                    self.scroll += 1;
                }
                PopupOutcome::Pending
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                PopupOutcome::Pending
            }
            _ => PopupOutcome::Pending,
        }
    }
}
