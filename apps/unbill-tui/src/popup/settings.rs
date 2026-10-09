use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Paragraph},
};
use unbill_console::model::{LedgerId, LedgerMeta, NewUser, NewUserName, User};

use super::{
    LedgerFingerprints, PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base,
    render_text_field,
};

// ---------------------------------------------------------------------------
// Public tab selector
// ---------------------------------------------------------------------------

// sirno:witness:unbill-tui:begin
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
    PeerSync,
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
    peer_input: TextInput,
    device_field: DeviceField,

    // Ledger tab state
    ledgers: Vec<LedgerMeta>,
    ledger_users_map: Vec<Vec<User>>,
    fingerprints: LedgerFingerprints,
    all_users: Vec<User>,
    ledger_cursor: usize,
    ledger_focus: LedgerFocus,
    ledger_sub_tab: LedgerSubTab,
    add_cursor: usize,
    creating_new_user: bool,
    create_user_input: TextInput,

    error: Option<String>,
}
// sirno:witness:unbill-tui:end

impl SettingsPopup {
    pub fn new(
        initial_tab: TopTab,
        device_id: String,
        all_users: Vec<User>,
        ledgers: Vec<LedgerMeta>,
        ledger_users_map: Vec<Vec<User>>,
        initial_ledger_cursor: usize,
    ) -> Self {
        Self {
            top_tab: initial_tab,
            device_id,
            peer_input: TextInput::new("Peer NodeId"),
            device_field: DeviceField::PeerSync,
            ledgers,
            ledger_users_map,
            fingerprints: LedgerFingerprints::new(),
            all_users,
            ledger_cursor: initial_ledger_cursor,
            ledger_focus: LedgerFocus::Selector,
            ledger_sub_tab: LedgerSubTab::Users,
            add_cursor: 0,
            creating_new_user: false,
            create_user_input: TextInput::new("Name"),
            error: None,
        }
    }

    /// Returns users from other ledgers on this device that are not yet in the selected ledger.
    fn addable_users(&self) -> Vec<User> {
        let ledger_users = self
            .ledger_users_map
            .get(self.ledger_cursor)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let ledger_ids: std::collections::HashSet<_> =
            ledger_users.iter().map(|u| u.user_id).collect();
        self.all_users
            .iter()
            .filter(|u| !ledger_ids.contains(&u.user_id))
            .cloned()
            .collect()
    }

    fn current_ledger_id(&self) -> Option<LedgerId> {
        self.ledgers.get(self.ledger_cursor).map(|l| l.ledger_id)
    }
}

// ---------------------------------------------------------------------------
// PopupView impl
// ---------------------------------------------------------------------------

impl PopupView for SettingsPopup {
    fn update_fingerprints(&mut self, fingerprints: &LedgerFingerprints) {
        self.fingerprints = fingerprints.clone();
    }
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
        .areas::<4>(inner);

        // Top tab bar
        let tab_cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas::<2>(rows[0]);

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
        let rows = Layout::vertical([
            Constraint::Length(1), // device ID info line
            Constraint::Length(3), // peer sync box
            Constraint::Min(0),
        ])
        .areas::<3>(content);

        // Device ID info (no box — read-only)
        frame.render_widget(
            Paragraph::new(format!("Device: {}", self.device_id))
                .style(Style::default().fg(Color::DarkGray)),
            rows[0],
        );

        // Peer Sync box
        let peer_block = Block::bordered()
            .title("Peer Sync")
            .border_style(focused_border_style(true));
        let peer_inner = peer_block.inner(rows[1]);
        frame.render_widget(peer_block, rows[1]);
        render_text_field(frame, peer_inner, &self.peer_input, true);

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                hint_row,
            );
        } else {
            frame.render_widget(
                Paragraph::new("[Tab] switch tab  [Enter] sync  [Esc] close")
                    .style(Style::default().fg(Color::DarkGray)),
                hint_row,
            );
        }
    }

    fn handle_device_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                self.top_tab = TopTab::Ledger;
                self.ledger_focus = LedgerFocus::Selector;
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::Enter => {
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
            KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = c;
                PopupOutcome::Pending
            }
            KeyCode::Char(c) => {
                self.error = None;
                self.peer_input.push(c);
                PopupOutcome::Pending
            }
            KeyCode::Backspace => {
                self.error = None;
                self.peer_input.pop();
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
        let fingerprint = self
            .current_ledger_id()
            .and_then(|id| self.fingerprints.get(&id));
        let text = match fingerprint {
            Some(Ok(text)) => text.as_str(),
            Some(Err(error)) => error.as_str(),
            None => "Select a ledger.",
        };
        let lines = fingerprint_lines(text, content.width);
        let height = u16::try_from(lines.len())
            .unwrap_or(u16::MAX)
            .saturating_add(3);
        let [selector_area, fingerprint_area, remainder] = Layout::vertical([
            Constraint::Length(
                u16::try_from(self.ledgers.len().clamp(1, 4))
                    .unwrap_or(4)
                    .saturating_add(2),
            ),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .areas(content);
        let [label, symbols, explanation] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(height.saturating_sub(3)),
            Constraint::Length(2),
        ])
        .areas(fingerprint_area);
        frame.render_widget(Paragraph::new("Ledger fingerprint  [c] copy"), label);
        frame.render_widget(Paragraph::new(lines), symbols);
        frame.render_widget(
            Paragraph::new(
                "Matching fingerprints suggest these devices have the same ledger state.",
            )
            .wrap(ratatui::widgets::Wrap { trim: true }),
            explanation,
        );
        let sections = [selector_area, remainder];

        let selector_focused = self.ledger_focus == LedgerFocus::Selector;
        let content_focused = self.ledger_focus == LedgerFocus::Content;

        // Ledger selector box
        let selector_block = Block::bordered()
            .title("Ledger")
            .border_style(focused_border_style(selector_focused));
        let selector_inner = selector_block.inner(sections[0]);
        frame.render_widget(selector_block, sections[0]);

        if self.ledgers.is_empty() {
            frame.render_widget(
                Paragraph::new("none").style(Style::default().fg(Color::DarkGray)),
                selector_inner,
            );
            frame.render_widget(
                Paragraph::new("[Tab] switch tab  [Esc] close")
                    .style(Style::default().fg(Color::DarkGray)),
                hint_row,
            );
            return;
        }

        for ((i, ledger), row) in self.ledgers.iter().enumerate().zip(selector_inner.rows()) {
            let is_selected = i == self.ledger_cursor;
            let style = if selector_focused && is_selected {
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

        // Content box — title shows active sub-tab
        let content_title = match self.ledger_sub_tab {
            LedgerSubTab::Users => "Users",
            LedgerSubTab::Invite => "Invite",
        };
        let content_block = Block::bordered()
            .title(content_title)
            .border_style(focused_border_style(content_focused));
        let content_inner = content_block.inner(sections[1]);
        frame.render_widget(content_block, sections[1]);

        // Sub-tab switcher row inside content box
        let inner_rows = Layout::vertical([
            Constraint::Length(1), // sub-tab switcher
            Constraint::Min(0),    // list / action area
        ])
        .areas::<2>(content_inner);

        let sub_tab_cols =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas::<2>(inner_rows[0]);

        let users_style = if content_focused && self.ledger_sub_tab == LedgerSubTab::Users {
            Style::default().add_modifier(Modifier::REVERSED)
        } else if self.ledger_sub_tab == LedgerSubTab::Users {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let invite_style = if content_focused && self.ledger_sub_tab == LedgerSubTab::Invite {
            Style::default().add_modifier(Modifier::REVERSED)
        } else if self.ledger_sub_tab == LedgerSubTab::Invite {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(" Users ").style(users_style),
            sub_tab_cols[0],
        );
        frame.render_widget(
            Paragraph::new(" Invite ").style(invite_style),
            sub_tab_cols[1],
        );

        let ledger_users = self
            .ledger_users_map
            .get(self.ledger_cursor)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let addable = self.addable_users();
        // Total addable rows = existing cross-ledger users + 1 "Create new" row.
        let list_area = inner_rows[1];

        match self.ledger_sub_tab {
            LedgerSubTab::Users => {
                let half = list_area.height / 2;
                let user_rows =
                    Layout::vertical([Constraint::Length(half.max(1)), Constraint::Min(0)])
                        .areas::<2>(list_area);

                // Top half: current ledger users (read-only)
                if ledger_users.is_empty() {
                    frame.render_widget(
                        Paragraph::new("no users yet").style(Style::default().fg(Color::DarkGray)),
                        user_rows[0],
                    );
                } else {
                    for (user, row) in ledger_users.iter().zip(user_rows[0].rows()) {
                        let short_id: String = user.user_id.to_string().chars().take(8).collect();
                        frame.render_widget(
                            Paragraph::new(format!("  {} ({})", user.display_name, short_id)),
                            row,
                        );
                    }
                }

                // Bottom half: addable cross-ledger users + "[ + New ]" row
                if self.creating_new_user {
                    // Show create-user text input
                    render_text_field(
                        frame,
                        user_rows[1],
                        &self.create_user_input,
                        content_focused,
                    );
                } else {
                    // Addable users followed by the "[ + New ]" item.
                    let items = addable.iter().map(Some).chain(std::iter::once(None));
                    for (i, (user, row)) in items.zip(user_rows[1].rows()).enumerate() {
                        let is_cursor = content_focused && i == self.add_cursor;
                        let style = if is_cursor {
                            Style::default().add_modifier(Modifier::REVERSED)
                        } else {
                            Style::default()
                        };
                        if let Some(user) = user {
                            let marker = if is_cursor { "+" } else { " " };
                            frame.render_widget(
                                Paragraph::new(format!("{} {}", marker, user.display_name))
                                    .style(style),
                                row,
                            );
                        } else {
                            // "Create new" row
                            frame.render_widget(Paragraph::new("[ + New ]").style(style), row);
                        }
                    }
                }
            }
            LedgerSubTab::Invite => {
                frame.render_widget(
                    Paragraph::new("Press Enter to generate an invite URL.")
                        .style(Style::default().fg(Color::DarkGray)),
                    list_area,
                );
            }
        }

        if let Some(err) = &self.error {
            frame.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                hint_row,
            );
        } else {
            let hint = if selector_focused {
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
        // When in "create new user" text-entry mode, intercept all keys.
        if self.creating_new_user {
            match key.code {
                KeyCode::Esc => {
                    self.creating_new_user = false;
                    self.create_user_input.value.clear();
                    self.error = None;
                }
                KeyCode::Enter => {
                    let name = self.create_user_input.value.trim().to_string();
                    if name.is_empty() {
                        self.error = Some("Enter a name".to_string());
                        return PopupOutcome::Pending;
                    }
                    let Some(ledger_id) = self.current_ledger_id() else {
                        return PopupOutcome::Pending;
                    };
                    self.creating_new_user = false;
                    self.create_user_input.value.clear();
                    self.error = None;
                    return PopupOutcome::Action(PopupAction::CreateUser {
                        ledger_id,
                        input: NewUserName { display_name: name },
                    });
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.create_user_input.push(c);
                }
                KeyCode::Backspace => {
                    self.create_user_input.pop();
                }
                _ => {}
            }
            return PopupOutcome::Pending;
        }

        match key.code {
            KeyCode::Char('c') => {
                if let Some(Ok(text)) = self
                    .current_ledger_id()
                    .and_then(|id| self.fingerprints.get(&id))
                {
                    PopupOutcome::CopyText(text.clone())
                } else {
                    PopupOutcome::Pending
                }
            }
            KeyCode::Tab => {
                match self.ledger_focus {
                    LedgerFocus::Selector => {
                        self.ledger_focus = LedgerFocus::Content;
                    }
                    LedgerFocus::Content => {
                        // Last area — advance to Device tab.
                        self.top_tab = TopTab::Device;
                        self.device_field = DeviceField::PeerSync;
                    }
                }
                self.error = None;
                PopupOutcome::Pending
            }
            KeyCode::BackTab => {
                match self.ledger_focus {
                    LedgerFocus::Selector => {
                        // First area — retreat to Device tab.
                        self.top_tab = TopTab::Device;
                        self.device_field = DeviceField::PeerSync;
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
                            self.ledger_cursor = self
                                .ledger_cursor
                                .saturating_add(1)
                                .min(self.ledgers.len().saturating_sub(1));
                            self.add_cursor = 0;
                        }
                    }
                    LedgerFocus::Content => {
                        if self.ledger_sub_tab == LedgerSubTab::Users {
                            // The final index is the "Create new" row.
                            self.add_cursor = self
                                .add_cursor
                                .saturating_add(1)
                                .min(self.addable_users().len());
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
                    let addable = self.addable_users();
                    let Some(ledger_id) = self.current_ledger_id() else {
                        return PopupOutcome::Pending;
                    };
                    self.error = None;
                    if self.add_cursor == addable.len() {
                        // "Create new" selected
                        self.creating_new_user = true;
                        PopupOutcome::Pending
                    } else {
                        let Some(user) = addable.get(self.add_cursor).cloned() else {
                            return PopupOutcome::Pending;
                        };
                        PopupOutcome::Action(PopupAction::AddUser {
                            ledger_id,
                            user: NewUser {
                                user_id: user.user_id,
                                display_name: user.display_name,
                            },
                        })
                    }
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn focused_border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn fingerprint_lines(text: &str, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for symbol in text.split(' ') {
        let candidate = if current.is_empty() {
            symbol.to_owned()
        } else {
            format!("{current} {symbol}")
        };
        if !current.is_empty() && Line::from(candidate.clone()).width() > usize::from(width) {
            lines.push(Line::from(std::mem::take(&mut current)));
            current = symbol.to_owned();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(Line::from(current));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_displays_and_copies_the_full_selected_ledger_fingerprint() {
        let ledger_id = LedgerId::from_u128(1);
        let mut popup = SettingsPopup::new(
            TopTab::Ledger,
            String::new(),
            vec![],
            vec![LedgerMeta {
                ledger_id,
                name: "Shared expenses".into(),
                currency: unbill_console::model::Currency::from_code("USD").unwrap(),
                created_at: unbill_console::model::Timestamp::from_millis(0),
                updated_at: unbill_console::model::Timestamp::from_millis(0),
            }],
            vec![vec![]],
            0,
        );
        let text = ["👩🏼‍❤️‍👩🏽"; 6].join(" ");
        popup.update_fingerprints(&LedgerFingerprints::from([(ledger_id, Ok(text.clone()))]));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
            .unwrap();
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert_eq!(rendered.matches("👩🏼‍❤️‍👩🏽").count(), 6);
        assert!(
            matches!(popup.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)), PopupOutcome::CopyText(value) if value == text)
        );
    }

    #[test]
    fn fingerprint_wraps_without_splitting_combined_symbols() {
        let text = ["👩🏼‍❤️‍👩🏽"; 6].join(" ");
        let lines = fingerprint_lines(&text, 12);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.width() <= 12));
        assert_eq!(
            lines
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
            text
        );
    }
}
