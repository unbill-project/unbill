use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};
use unbill_core::model::{NewUser, User};
use unbill_core::service::LocalUser;

use super::{PopupAction, PopupOutcome, PopupView, render_popup_base};

#[derive(PartialEq, Eq)]
enum SettingsTab {
    Users,
    Invite,
}

pub struct LedgerSettingsPopup {
    ledger_id: String,
    ledger_users: Vec<User>,
    /// Local device users not yet in the ledger.
    local_users: Vec<LocalUser>,
    tab: SettingsTab,
    add_cursor: usize,
}

impl LedgerSettingsPopup {
    pub fn new(
        ledger_id: String,
        ledger_users: Vec<User>,
        all_local_users: Vec<LocalUser>,
    ) -> Self {
        let ledger_user_ids: std::collections::HashSet<_> =
            ledger_users.iter().map(|u| u.user_id).collect();
        let local_users: Vec<LocalUser> = all_local_users
            .into_iter()
            .filter(|u| !ledger_user_ids.contains(&u.user_id))
            .collect();
        Self {
            ledger_id,
            ledger_users,
            local_users,
            tab: SettingsTab::Users,
            add_cursor: 0,
        }
    }
}

impl PopupView for LedgerSettingsPopup {
    fn title(&self) -> &str {
        "Ledger Settings"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Length(1), // tab bar
            Constraint::Length(1), // separator / spacer
            Constraint::Min(0),    // list / content
            Constraint::Length(1), // hint
        ])
        .split(inner);

        // Tab bar
        let tab_cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);

        let users_style = if self.tab == SettingsTab::Users {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let invite_style = if self.tab == SettingsTab::Invite {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        frame.render_widget(Paragraph::new(" Users ").style(users_style), tab_cols[0]);
        frame.render_widget(Paragraph::new(" Invite ").style(invite_style), tab_cols[1]);

        match self.tab {
            SettingsTab::Users => {
                // Show ledger users at top, then "Add from device" below
                if self.ledger_users.is_empty() {
                    frame.render_widget(
                        Paragraph::new("no users in this ledger")
                            .style(Style::default().fg(Color::DarkGray)),
                        rows[2],
                    );
                } else {
                    // Split content area: ledger users / add users
                    let half = rows[2].height / 2;
                    let content_rows =
                        Layout::vertical([Constraint::Length(half.max(1)), Constraint::Min(0)])
                            .split(rows[2]);

                    for (i, user) in self.ledger_users.iter().enumerate() {
                        if i >= content_rows[0].height as usize {
                            break;
                        }
                        let row = Rect {
                            x: content_rows[0].x,
                            y: content_rows[0].y + i as u16,
                            width: content_rows[0].width,
                            height: 1,
                        };
                        let short_id = &user.user_id.to_string()[..8];
                        frame.render_widget(
                            Paragraph::new(format!("  {} ({})", user.display_name, short_id)),
                            row,
                        );
                    }

                    if self.local_users.is_empty() {
                        frame.render_widget(
                            Paragraph::new("no device users to add")
                                .style(Style::default().fg(Color::DarkGray)),
                            content_rows[1],
                        );
                    } else {
                        for (i, user) in self.local_users.iter().enumerate() {
                            if i >= content_rows[1].height as usize {
                                break;
                            }
                            let row = Rect {
                                x: content_rows[1].x,
                                y: content_rows[1].y + i as u16,
                                width: content_rows[1].width,
                                height: 1,
                            };
                            let is_cursor = i == self.add_cursor;
                            let style = if is_cursor {
                                Style::default().add_modifier(Modifier::REVERSED)
                            } else {
                                Style::default()
                            };
                            let marker = if is_cursor { "+" } else { " " };
                            frame.render_widget(
                                Paragraph::new(format!("{} {}", marker, user.display_name))
                                    .style(style),
                                row,
                            );
                        }
                    }
                }
                frame.render_widget(
                    Paragraph::new("[j/k] move  [Enter] add user  [h/l] switch tab  [Esc] close")
                        .style(Style::default().fg(Color::DarkGray)),
                    rows[3],
                );
            }
            SettingsTab::Invite => {
                frame.render_widget(
                    Paragraph::new("Press Enter to generate an invite URL for this ledger.")
                        .style(Style::default().fg(Color::DarkGray)),
                    rows[2],
                );
                frame.render_widget(
                    Paragraph::new("[Enter] generate URL  [h/l] switch tab  [Esc] close")
                        .style(Style::default().fg(Color::DarkGray)),
                    rows[3],
                );
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            KeyCode::Char('h') => {
                self.tab = SettingsTab::Users;
                PopupOutcome::Pending
            }
            KeyCode::Char('l') => {
                self.tab = SettingsTab::Invite;
                PopupOutcome::Pending
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.tab == SettingsTab::Users && !self.local_users.is_empty() {
                    self.add_cursor = (self.add_cursor + 1).min(self.local_users.len() - 1);
                }
                PopupOutcome::Pending
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.tab == SettingsTab::Users {
                    self.add_cursor = self.add_cursor.saturating_sub(1);
                }
                PopupOutcome::Pending
            }
            KeyCode::Enter => match self.tab {
                SettingsTab::Users => {
                    if !self.local_users.is_empty() {
                        let local = &self.local_users[self.add_cursor];
                        PopupOutcome::Action(PopupAction::AddUser {
                            ledger_id: self.ledger_id.clone(),
                            user: NewUser {
                                user_id: local.user_id,
                                display_name: local.display_name.clone(),
                            },
                        })
                    } else {
                        PopupOutcome::Pending
                    }
                }
                SettingsTab::Invite => PopupOutcome::Action(PopupAction::GenerateInvite {
                    ledger_id: self.ledger_id.clone(),
                }),
            },
            _ => PopupOutcome::Pending,
        }
    }
}
