use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};

use super::{PopupAction, PopupOutcome, PopupView, TextInput, render_popup_base, render_text_field};

// ---------------------------------------------------------------------------
// InvitePopup — two-tab Invite / Join
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
enum InviteTab {
    Invite,
    Join,
}

pub struct InvitePopup {
    ledger_id: String,
    tab: InviteTab,
    join_url: TextInput,
    error: Option<String>,
}

impl InvitePopup {
    pub fn new(ledger_id: String) -> Self {
        Self {
            ledger_id,
            tab: InviteTab::Invite,
            join_url: TextInput::new("URL"),
            error: None,
        }
    }
}

impl PopupView for InvitePopup {
    fn title(&self) -> &str {
        "Invite / Join"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Length(1), // tab bar
            Constraint::Length(1), // spacer
            Constraint::Min(0),    // content
            Constraint::Length(1), // hint
        ])
        .split(inner);

        // Tab bar
        let tab_cols =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[0]);
        let invite_style = if self.tab == InviteTab::Invite {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let join_style = if self.tab == InviteTab::Join {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(Paragraph::new(" Invite ").style(invite_style), tab_cols[0]);
        frame.render_widget(Paragraph::new(" Join ").style(join_style), tab_cols[1]);

        match self.tab {
            InviteTab::Invite => {
                frame.render_widget(
                    Paragraph::new(format!(
                        "Ledger: {}",
                        &self.ledger_id[..8.min(self.ledger_id.len())]
                    ))
                    .style(Style::default().fg(Color::DarkGray)),
                    rows[2],
                );
                frame.render_widget(
                    Paragraph::new("[Enter] generate URL  [h/l] switch  [Esc] close")
                        .style(Style::default().fg(Color::DarkGray)),
                    rows[3],
                );
            }
            InviteTab::Join => {
                render_text_field(frame, rows[2], &self.join_url, true);
                if let Some(err) = &self.error {
                    frame.render_widget(
                        Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                        rows[3],
                    );
                } else {
                    frame.render_widget(
                        Paragraph::new("[Enter] join  [h/l] switch  [Esc] close")
                            .style(Style::default().fg(Color::DarkGray)),
                        rows[3],
                    );
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            KeyCode::Char('h') => {
                self.tab = InviteTab::Invite;
                PopupOutcome::Pending
            }
            KeyCode::Char('l') => {
                self.tab = InviteTab::Join;
                PopupOutcome::Pending
            }
            KeyCode::Enter => match self.tab {
                InviteTab::Invite => PopupOutcome::Action(PopupAction::GenerateInvite {
                    ledger_id: self.ledger_id.clone(),
                }),
                InviteTab::Join => {
                    let url = self.join_url.value.trim().to_string();
                    if url.is_empty() {
                        self.error = Some("Enter a URL".to_string());
                        PopupOutcome::Pending
                    } else {
                        self.error = None;
                        PopupOutcome::Action(PopupAction::JoinLedger { url })
                    }
                }
            },
            KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = c;
                PopupOutcome::Pending
            }
            KeyCode::Char(c) => {
                if self.tab == InviteTab::Join {
                    self.error = None;
                    self.join_url.push(c);
                }
                PopupOutcome::Pending
            }
            KeyCode::Backspace => {
                if self.tab == InviteTab::Join {
                    self.error = None;
                    self.join_url.pop();
                }
                PopupOutcome::Pending
            }
            _ => PopupOutcome::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// InviteResultPopup — shows the generated URL
// ---------------------------------------------------------------------------

pub struct InviteResultPopup {
    url: String,
}

impl InviteResultPopup {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl PopupView for InviteResultPopup {
    fn title(&self) -> &str {
        "Invite URL"
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let inner = render_popup_base(frame, area, self.title());

        let rows = Layout::vertical([
            Constraint::Min(0),    // url
            Constraint::Length(1), // hint
        ])
        .split(inner);

        frame.render_widget(Paragraph::new(self.url.as_str()).wrap(ratatui::widgets::Wrap { trim: false }), rows[0]);
        frame.render_widget(
            Paragraph::new("[Esc] close").style(Style::default().fg(Color::DarkGray)),
            rows[1],
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome {
        match key.code {
            KeyCode::Esc => PopupOutcome::Cancelled,
            _ => PopupOutcome::Pending,
        }
    }
}
