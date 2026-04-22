use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};
use unbill_core::service::LocalUser;

use super::{
    PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field,
};

#[derive(PartialEq, Eq)]
enum Field {
    AddUser,
    ImportUser,
    PeerSync,
    ShareUser,
}

pub struct DevicePopup {
    device_id: String,
    saved_users: Vec<LocalUser>,
    add_user_input: TextInput,
    import_user_input: TextInput,
    peer_input: TextInput,
    focused: Field,
    share_cursor: usize,
    error: Option<String>,
}

impl DevicePopup {
    pub fn new(device_id: String, saved_users: Vec<LocalUser>) -> Self {
        Self {
            device_id,
            saved_users,
            add_user_input: TextInput::new("New user"),
            import_user_input: TextInput::new("Import URL"),
            peer_input: TextInput::new("Peer NodeId"),
            focused: Field::AddUser,
            share_cursor: 0,
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

        let saved_count = self.saved_users.len().max(1) as u16;

        let rows = Layout::vertical([
            Constraint::Length(1),           // "Device ID:" label
            Constraint::Length(1),           // device id value
            Constraint::Length(1),           // spacer
            Constraint::Length(1),           // "Saved Users:" label
            Constraint::Length(saved_count), // saved user list
            Constraint::Length(1),           // add user input
            Constraint::Length(1),           // import user input
            Constraint::Length(1),           // spacer
            Constraint::Length(1),           // peer sync input
            Constraint::Length(1),           // error / hint
        ])
        .split(inner);

        frame.render_widget(
            Paragraph::new("Device ID:").style(Style::default().fg(Color::DarkGray)),
            rows[0],
        );
        frame.render_widget(Paragraph::new(self.device_id.as_str()), rows[1]);

        let users_label_style = if self.focused == Field::ShareUser {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new("Saved Users:").style(users_label_style),
            rows[3],
        );

        if self.saved_users.is_empty() {
            frame.render_widget(
                Paragraph::new("  none").style(Style::default().fg(Color::DarkGray)),
                rows[4],
            );
        } else {
            for (i, user) in self.saved_users.iter().enumerate() {
                if i >= rows[4].height as usize {
                    break;
                }
                let row = Rect {
                    x: rows[4].x,
                    y: rows[4].y + i as u16,
                    width: rows[4].width,
                    height: 1,
                };
                let is_cursor = self.focused == Field::ShareUser && i == self.share_cursor;
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

        render_text_field(
            frame,
            rows[5],
            &self.add_user_input,
            self.focused == Field::AddUser,
        );
        render_text_field(
            frame,
            rows[6],
            &self.import_user_input,
            self.focused == Field::ImportUser,
        );
        render_text_field(
            frame,
            rows[8],
            &self.peer_input,
            self.focused == Field::PeerSync,
        );

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                rows[9],
            );
        } else {
            let hint = match self.focused {
                Field::ShareUser => "[j/k] move  [Enter] share  [Tab] switch  [Esc] close",
                _ => "[Tab] switch  [Enter] confirm  [Esc] close",
            };
            frame.render_widget(
                Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
                rows[9],
            );
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            KeyCode::Tab if self.focused != Field::ShareUser => {
                self.focused = match self.focused {
                    Field::AddUser => Field::ImportUser,
                    Field::ImportUser => Field::PeerSync,
                    Field::PeerSync => Field::ShareUser,
                    Field::ShareUser => unreachable!(),
                };
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::Tab => {
                // ShareUser → AddUser
                self.focused = Field::AddUser;
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::BackTab if self.focused != Field::ShareUser => {
                self.focused = match self.focused {
                    Field::AddUser => Field::ShareUser,
                    Field::ImportUser => Field::AddUser,
                    Field::PeerSync => Field::ImportUser,
                    Field::ShareUser => unreachable!(),
                };
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::BackTab => {
                // ShareUser → PeerSync
                self.focused = Field::PeerSync;
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::Char('j') | KeyCode::Down if self.focused == Field::ShareUser => {
                if !self.saved_users.is_empty() {
                    self.share_cursor = (self.share_cursor + 1).min(self.saved_users.len() - 1);
                }
                PopupOutcome::Pending
            }
            KeyCode::Char('k') | KeyCode::Up if self.focused == Field::ShareUser => {
                self.share_cursor = self.share_cursor.saturating_sub(1);
                PopupOutcome::Pending
            }
            KeyCode::Enter => match self.focused {
                Field::AddUser => {
                    let name = self.add_user_input.value.trim().to_string();
                    if name.is_empty() {
                        self.error = Some("Enter a name".to_string());
                        return PopupOutcome::Pending;
                    }
                    self.error = None;
                    PopupOutcome::Action(PopupAction::AddLocalUser { display_name: name })
                }
                Field::ImportUser => {
                    let url = self.import_user_input.value.trim().to_string();
                    if url.is_empty() {
                        self.error = Some("Enter a user share URL".to_string());
                        return PopupOutcome::Pending;
                    }
                    self.error = None;
                    PopupOutcome::Action(PopupAction::ImportLocalUser { url })
                }
                Field::PeerSync => {
                    let peer_str = self.peer_input.value.trim().to_string();
                    if peer_str.is_empty() {
                        self.error = Some("Enter a peer NodeId".to_string());
                        return PopupOutcome::Pending;
                    }
                    self.error = None;
                    PopupOutcome::Action(PopupAction::SyncOnce {
                        peer_node_id: peer_str,
                    })
                }
                Field::ShareUser => {
                    if self.saved_users.is_empty() {
                        self.error = Some("No saved users to share".to_string());
                        return PopupOutcome::Pending;
                    }
                    let user_id = self.saved_users[self.share_cursor].user_id.to_string();
                    self.error = None;
                    PopupOutcome::Action(PopupAction::ShareLocalUser { user_id })
                }
            },
            KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = c;
                PopupOutcome::Pending
            }
            KeyCode::Char(c) => {
                self.error = None;
                match self.focused {
                    Field::AddUser => self.add_user_input.push(c),
                    Field::ImportUser => self.import_user_input.push(c),
                    Field::PeerSync => self.peer_input.push(c),
                    Field::ShareUser => {}
                }
                PopupOutcome::Pending
            }
            KeyCode::Backspace => {
                self.error = None;
                match self.focused {
                    Field::AddUser => self.add_user_input.pop(),
                    Field::ImportUser => self.import_user_input.pop(),
                    Field::PeerSync => self.peer_input.pop(),
                    Field::ShareUser => {}
                }
                PopupOutcome::Pending
            }
            _ => PopupOutcome::Pending,
        }
    }
}
