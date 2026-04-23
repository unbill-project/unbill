use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};
use unbill_core::model::{LedgerMeta, NewUser, User};
use unbill_core::service::LocalUser;

use super::{
    PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field,
};

// ---------------------------------------------------------------------------
// Public tab selector
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum TopTab {
    Device,
    Ledger,
}

// ---------------------------------------------------------------------------
// Device tab internals
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
enum DeviceField {
    AddUser,
    ImportUser,
    PeerSync,
    ShareUser,
}

// ---------------------------------------------------------------------------
// Ledger tab internals
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
enum LedgerFocus {
    Selector,
    Content,
}

#[derive(PartialEq, Eq)]
enum LedgerSubTab {
    Users,
    Invite,
}

// ---------------------------------------------------------------------------
// SettingsPopup
// ---------------------------------------------------------------------------

pub struct SettingsPopup {
    top_tab: TopTab,

    // Device tab state
    device_id: String,
    saved_users: Vec<LocalUser>,
    add_user_input: TextInput,
    import_user_input: TextInput,
    peer_input: TextInput,
    device_field: DeviceField,
    share_cursor: usize,

    // Ledger tab state
    ledgers: Vec<LedgerMeta>,
    ledger_users_map: Vec<Vec<User>>,
    all_local_users: Vec<LocalUser>,
    ledger_cursor: usize,
    ledger_focus: LedgerFocus,
    ledger_sub_tab: LedgerSubTab,
    add_cursor: usize,

    error: Option<String>,
}

impl SettingsPopup {
    pub fn new(
        initial_tab: TopTab,
        device_id: String,
        saved_users: Vec<LocalUser>,
        ledgers: Vec<LedgerMeta>,
        ledger_users_map: Vec<Vec<User>>,
        all_local_users: Vec<LocalUser>,
        initial_ledger_cursor: usize,
    ) -> Self {
        Self {
            top_tab: initial_tab,
            device_id,
            saved_users,
            add_user_input: TextInput::new("New user"),
            import_user_input: TextInput::new("Import URL"),
            peer_input: TextInput::new("Peer NodeId"),
            device_field: DeviceField::AddUser,
            share_cursor: 0,
            ledgers,
            ledger_users_map,
            all_local_users,
            ledger_cursor: initial_ledger_cursor,
            ledger_focus: LedgerFocus::Selector,
            ledger_sub_tab: LedgerSubTab::Users,
            add_cursor: 0,
            error: None,
        }
    }

    /// Returns local users that are not yet in the currently selected ledger.
    fn addable_local_users(&self) -> Vec<LocalUser> {
        let ledger_users = self
            .ledger_users_map
            .get(self.ledger_cursor)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let ledger_ids: std::collections::HashSet<_> =
            ledger_users.iter().map(|u| u.user_id).collect();
        self.all_local_users
            .iter()
            .filter(|u| !ledger_ids.contains(&u.user_id))
            .cloned()
            .collect()
    }

    fn current_ledger_id(&self) -> Option<String> {
        self.ledgers
            .get(self.ledger_cursor)
            .map(|l| l.ledger_id.to_string())
    }
}

// ---------------------------------------------------------------------------
// PopupView impl
// ---------------------------------------------------------------------------

impl PopupView for SettingsPopup {
    fn title(&self) -> &str {
        "Settings"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Length(1), // top tab bar
            Constraint::Length(1), // spacer
            Constraint::Min(0),    // content
            Constraint::Length(1), // hint
        ])
        .split(inner);

        // Top tab bar
        let tab_cols =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[0]);

        let device_style = if self.top_tab == TopTab::Device {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let ledger_style = if self.top_tab == TopTab::Ledger {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(" Device Settings ").style(device_style),
            tab_cols[0],
        );
        frame.render_widget(
            Paragraph::new(" Ledger Settings ").style(ledger_style),
            tab_cols[1],
        );

        match self.top_tab {
            TopTab::Device => self.render_device_tab(frame, rows[2], rows[3]),
            TopTab::Ledger => self.render_ledger_tab(frame, rows[2], rows[3]),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        if key.code == KeyCode::Esc {
            return PopupOutcome::Cancelled;
        }
        match self.top_tab {
            TopTab::Device => self.handle_device_key(key),
            TopTab::Ledger => self.handle_ledger_key(key),
        }
    }
}

// ---------------------------------------------------------------------------
// Device tab
// ---------------------------------------------------------------------------

impl SettingsPopup {
    fn render_device_tab(&self, frame: &mut Frame, content: Rect, hint_row: Rect) {
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
        ])
        .split(content);

        frame.render_widget(
            Paragraph::new("Device ID:").style(Style::default().fg(Color::DarkGray)),
            rows[0],
        );
        frame.render_widget(Paragraph::new(self.device_id.as_str()), rows[1]);

        let users_label_style = if self.device_field == DeviceField::ShareUser {
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
                let is_cursor =
                    self.device_field == DeviceField::ShareUser && i == self.share_cursor;
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
            self.device_field == DeviceField::AddUser,
        );
        render_text_field(
            frame,
            rows[6],
            &self.import_user_input,
            self.device_field == DeviceField::ImportUser,
        );
        render_text_field(
            frame,
            rows[8],
            &self.peer_input,
            self.device_field == DeviceField::PeerSync,
        );

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                hint_row,
            );
        } else {
            let hint = match self.device_field {
                DeviceField::ShareUser => {
                    "[j/k] move  [Enter] share  [Tab] next tab  [Esc] close"
                }
                _ => "[Tab] switch field  [Enter] confirm  [Esc] close",
            };
            frame.render_widget(
                Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
                hint_row,
            );
        }
    }

    fn handle_device_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Tab if self.device_field == DeviceField::ShareUser => {
                // Last field — advance to Ledger tab.
                self.top_tab = TopTab::Ledger;
                self.ledger_focus = LedgerFocus::Selector;
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::Tab => {
                self.device_field = match self.device_field {
                    DeviceField::AddUser => DeviceField::ImportUser,
                    DeviceField::ImportUser => DeviceField::PeerSync,
                    DeviceField::PeerSync => DeviceField::ShareUser,
                    DeviceField::ShareUser => unreachable!(),
                };
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::BackTab if self.device_field == DeviceField::AddUser => {
                // First field — retreat to Ledger tab.
                self.top_tab = TopTab::Ledger;
                self.ledger_focus = LedgerFocus::Content;
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::BackTab => {
                self.device_field = match self.device_field {
                    DeviceField::AddUser => unreachable!(),
                    DeviceField::ImportUser => DeviceField::AddUser,
                    DeviceField::PeerSync => DeviceField::ImportUser,
                    DeviceField::ShareUser => DeviceField::PeerSync,
                };
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::Char('j') | KeyCode::Down
                if self.device_field == DeviceField::ShareUser =>
            {
                if !self.saved_users.is_empty() {
                    self.share_cursor =
                        (self.share_cursor + 1).min(self.saved_users.len() - 1);
                }
                PopupOutcome::Pending
            }
            KeyCode::Char('k') | KeyCode::Up
                if self.device_field == DeviceField::ShareUser =>
            {
                self.share_cursor = self.share_cursor.saturating_sub(1);
                PopupOutcome::Pending
            }
            KeyCode::Enter => match self.device_field {
                DeviceField::AddUser => {
                    let name = self.add_user_input.value.trim().to_string();
                    if name.is_empty() {
                        self.error = Some("Enter a name".to_string());
                        return PopupOutcome::Pending;
                    }
                    self.error = None;
                    PopupOutcome::Action(PopupAction::AddLocalUser { display_name: name })
                }
                DeviceField::ImportUser => {
                    let url = self.import_user_input.value.trim().to_string();
                    if url.is_empty() {
                        self.error = Some("Enter a user share URL".to_string());
                        return PopupOutcome::Pending;
                    }
                    self.error = None;
                    PopupOutcome::Action(PopupAction::ImportLocalUser { url })
                }
                DeviceField::PeerSync => {
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
                DeviceField::ShareUser => {
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
                match self.device_field {
                    DeviceField::AddUser => self.add_user_input.push(c),
                    DeviceField::ImportUser => self.import_user_input.push(c),
                    DeviceField::PeerSync => self.peer_input.push(c),
                    DeviceField::ShareUser => {}
                }
                PopupOutcome::Pending
            }
            KeyCode::Backspace => {
                self.error = None;
                match self.device_field {
                    DeviceField::AddUser => self.add_user_input.pop(),
                    DeviceField::ImportUser => self.import_user_input.pop(),
                    DeviceField::PeerSync => self.peer_input.pop(),
                    DeviceField::ShareUser => {}
                }
                PopupOutcome::Pending
            }
            _ => PopupOutcome::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// Ledger tab
// ---------------------------------------------------------------------------

impl SettingsPopup {
    fn render_ledger_tab(&self, frame: &mut Frame, content: Rect, hint_row: Rect) {
        let selector_height = (self.ledgers.len().max(1) as u16).min(4);

        let sections = Layout::vertical([
            Constraint::Length(selector_height),
            Constraint::Length(1), // spacer
            Constraint::Min(0),    // ledger content
        ])
        .split(content);

        // Ledger selector
        if self.ledgers.is_empty() {
            frame.render_widget(
                Paragraph::new("no ledgers").style(Style::default().fg(Color::DarkGray)),
                sections[0],
            );
            frame.render_widget(
                Paragraph::new("[Tab] switch tab  [Esc] close")
                    .style(Style::default().fg(Color::DarkGray)),
                hint_row,
            );
            return;
        }

        for (i, ledger) in self.ledgers.iter().enumerate() {
            if i >= sections[0].height as usize {
                break;
            }
            let row = Rect {
                x: sections[0].x,
                y: sections[0].y + i as u16,
                width: sections[0].width,
                height: 1,
            };
            let is_selected = i == self.ledger_cursor;
            let style = if self.ledger_focus == LedgerFocus::Selector && is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else if is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let marker = if is_selected { ">" } else { " " };
            frame.render_widget(
                Paragraph::new(format!("{} {}", marker, ledger.name)).style(style),
                row,
            );
        }

        // Ledger content area: sub-tab bar + list
        let content_rows = Layout::vertical([
            Constraint::Length(1), // sub-tab bar
            Constraint::Length(1), // spacer
            Constraint::Min(0),    // list / action area
        ])
        .split(sections[2]);

        let sub_tab_cols =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(content_rows[0]);

        let users_style = if self.ledger_sub_tab == LedgerSubTab::Users {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let invite_style = if self.ledger_sub_tab == LedgerSubTab::Invite {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(Paragraph::new(" Users ").style(users_style), sub_tab_cols[0]);
        frame.render_widget(
            Paragraph::new(" Invite ").style(invite_style),
            sub_tab_cols[1],
        );

        let ledger_users = self
            .ledger_users_map
            .get(self.ledger_cursor)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let addable = self.addable_local_users();

        match self.ledger_sub_tab {
            LedgerSubTab::Users => {
                if ledger_users.is_empty() && addable.is_empty() {
                    frame.render_widget(
                        Paragraph::new("no users in this ledger")
                            .style(Style::default().fg(Color::DarkGray)),
                        content_rows[2],
                    );
                } else {
                    let half = content_rows[2].height / 2;
                    let user_rows = Layout::vertical([
                        Constraint::Length(half.max(1)),
                        Constraint::Min(0),
                    ])
                    .split(content_rows[2]);

                    for (i, user) in ledger_users.iter().enumerate() {
                        if i >= user_rows[0].height as usize {
                            break;
                        }
                        let row = Rect {
                            x: user_rows[0].x,
                            y: user_rows[0].y + i as u16,
                            width: user_rows[0].width,
                            height: 1,
                        };
                        let short_id = &user.user_id.to_string()[..8];
                        frame.render_widget(
                            Paragraph::new(format!("  {} ({})", user.display_name, short_id)),
                            row,
                        );
                    }

                    if addable.is_empty() {
                        frame.render_widget(
                            Paragraph::new("no device users to add")
                                .style(Style::default().fg(Color::DarkGray)),
                            user_rows[1],
                        );
                    } else {
                        for (i, user) in addable.iter().enumerate() {
                            if i >= user_rows[1].height as usize {
                                break;
                            }
                            let row = Rect {
                                x: user_rows[1].x,
                                y: user_rows[1].y + i as u16,
                                width: user_rows[1].width,
                                height: 1,
                            };
                            let is_cursor =
                                self.ledger_focus == LedgerFocus::Content && i == self.add_cursor;
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
            }
            LedgerSubTab::Invite => {
                frame.render_widget(
                    Paragraph::new("Press Enter to generate an invite URL for this ledger.")
                        .style(Style::default().fg(Color::DarkGray)),
                    content_rows[2],
                );
            }
        }

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                hint_row,
            );
        } else {
            let hint = if self.ledger_focus == LedgerFocus::Selector {
                "[j/k] select ledger  [Tab] to content  [Esc] close"
            } else {
                "[j/k] move  [Enter] confirm  [h/l] sub-tab  [Tab] next tab  [Esc] close"
            };
            frame.render_widget(
                Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
                hint_row,
            );
        }
    }

    fn handle_ledger_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Tab => {
                match self.ledger_focus {
                    LedgerFocus::Selector => {
                        self.ledger_focus = LedgerFocus::Content;
                    }
                    LedgerFocus::Content => {
                        // Last area — advance to Device tab.
                        self.top_tab = TopTab::Device;
                        self.device_field = DeviceField::AddUser;
                    }
                }
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::BackTab => {
                match self.ledger_focus {
                    LedgerFocus::Selector => {
                        // First area — retreat to Device tab (last field).
                        self.top_tab = TopTab::Device;
                        self.device_field = DeviceField::ShareUser;
                    }
                    LedgerFocus::Content => {
                        self.ledger_focus = LedgerFocus::Selector;
                    }
                }
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.ledger_focus {
                    LedgerFocus::Selector => {
                        if !self.ledgers.is_empty() {
                            self.ledger_cursor =
                                (self.ledger_cursor + 1).min(self.ledgers.len() - 1);
                            self.add_cursor = 0;
                        }
                    }
                    LedgerFocus::Content => {
                        if self.ledger_sub_tab == LedgerSubTab::Users {
                            let len = self.addable_local_users().len();
                            if len > 0 {
                                self.add_cursor = (self.add_cursor + 1).min(len - 1);
                            }
                        }
                    }
                }
                PopupOutcome::Pending
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.ledger_focus {
                    LedgerFocus::Selector => {
                        self.ledger_cursor = self.ledger_cursor.saturating_sub(1);
                        self.add_cursor = 0;
                    }
                    LedgerFocus::Content => {
                        if self.ledger_sub_tab == LedgerSubTab::Users {
                            self.add_cursor = self.add_cursor.saturating_sub(1);
                        }
                    }
                }
                PopupOutcome::Pending
            }
            KeyCode::Char('h') if self.ledger_focus == LedgerFocus::Content => {
                self.ledger_sub_tab = LedgerSubTab::Users;
                self.add_cursor = 0;
                PopupOutcome::Pending
            }
            KeyCode::Char('l') if self.ledger_focus == LedgerFocus::Content => {
                self.ledger_sub_tab = LedgerSubTab::Invite;
                PopupOutcome::Pending
            }
            KeyCode::Enter if self.ledger_focus == LedgerFocus::Selector => {
                self.ledger_focus = LedgerFocus::Content;
                PopupOutcome::Pending
            }
            KeyCode::Enter => match self.ledger_sub_tab {
                LedgerSubTab::Users => {
                    let addable = self.addable_local_users();
                    if addable.is_empty() {
                        self.error = Some("No device users to add".to_string());
                        return PopupOutcome::Pending;
                    }
                    let idx = self.add_cursor.min(addable.len() - 1);
                    let local = addable[idx].clone();
                    let Some(ledger_id) = self.current_ledger_id() else {
                        return PopupOutcome::Pending;
                    };
                    self.error = None;
                    PopupOutcome::Action(PopupAction::AddUser {
                        ledger_id,
                        user: NewUser {
                            user_id: local.user_id,
                            display_name: local.display_name,
                        },
                    })
                }
                LedgerSubTab::Invite => {
                    let Some(ledger_id) = self.current_ledger_id() else {
                        return PopupOutcome::Pending;
                    };
                    PopupOutcome::Action(PopupAction::GenerateInvite { ledger_id })
                }
            },
            _ => PopupOutcome::Pending,
        }
    }
}
