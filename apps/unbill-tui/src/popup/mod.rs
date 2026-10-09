use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Clear, Paragraph},
};

use unbill_console::model::{LedgerId, NewBill, NewUser, NewUserName};

pub mod create_ledger;
pub mod invite;
pub mod settings;

pub type LedgerFingerprints = std::collections::HashMap<LedgerId, Result<String, String>>;

// sirno:witness:unbill-tui:begin
/// Trait implemented by every popup view.
pub trait PopupView: Send {
    fn title(&self) -> &str;
    fn render(&self, frame: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome;
    fn update_fingerprints(&mut self, _fingerprints: &LedgerFingerprints) {}
}

/// Outcome returned by a popup after handling a key event.
#[allow(dead_code)]
pub enum PopupOutcome {
    /// The popup stays open; no further action.
    Pending,
    /// Close the popup without performing any action.
    Cancelled,
    /// Close the popup and execute the given action against the service.
    Action(PopupAction),
    /// Replace this popup with the given one.
    OpenNext(Box<dyn PopupView>),
    /// Request terminal clipboard copy while keeping the popup open.
    CopyText(String),
}

/// Describes the service mutation to perform after a popup confirms.
#[allow(dead_code)]
pub enum PopupAction {
    CreateLedger {
        name: String,
        currency: String,
    },
    AddBill {
        ledger_id: LedgerId,
        bill: NewBill,
    },
    AddUser {
        ledger_id: LedgerId,
        user: NewUser,
    },
    CreateUser {
        ledger_id: LedgerId,
        input: NewUserName,
    },
    GenerateInvite {
        ledger_id: LedgerId,
    },
    JoinLedger {
        url: String,
    },
    SyncOnce {
        peer_node_id: String,
    },
}
// sirno:witness:unbill-tui:end

// ---------------------------------------------------------------------------
// TextInput helper
// ---------------------------------------------------------------------------

/// A single labelled text input field.
pub struct TextInput {
    pub label: &'static str,
    pub value: String,
}

impl TextInput {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            value: String::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_value(label: &'static str, value: String) -> Self {
        Self { label, value }
    }

    pub fn push(&mut self, c: char) {
        self.value.push(c);
    }

    pub fn pop(&mut self) {
        self.value.pop();
    }
}

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

/// Returns a centred `Rect` that is `percent_x`% wide and `percent_y`% tall
/// of the given `r`.
// sirno:witness:unbill-tui:begin
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    use ratatui::layout::{Constraint, Layout};

    #[allow(
        clippy::arithmetic_side_effects,
        reason = "u16 dimensions times percentages up to 100 fit u32; the result fits u16"
    )]
    let popup_height = (u32::from(r.height) * u32::from(percent_y.min(100)) / 100) as u16;
    #[allow(
        clippy::arithmetic_side_effects,
        reason = "u16 dimensions times percentages up to 100 fit u32; the result fits u16"
    )]
    let popup_width = (u32::from(r.width) * u32::from(percent_x.min(100)) / 100) as u16;

    let vertical = Layout::vertical([
        Constraint::Length((r.height.saturating_sub(popup_height)) / 2),
        Constraint::Length(popup_height),
        Constraint::Min(0),
    ])
    .areas::<3>(r);

    let horizontal = Layout::horizontal([
        Constraint::Length((r.width.saturating_sub(popup_width)) / 2),
        Constraint::Length(popup_width),
        Constraint::Min(0),
    ])
    .areas::<3>(vertical[1]);

    horizontal[1]
}
// sirno:witness:unbill-tui:end

/// Clears `area`, draws a bordered block with `title`, and returns the inner
/// area available for content.
pub fn render_popup_base(frame: &mut Frame, area: Rect, title: &str) -> Rect {
    let block = Block::bordered()
        .title(title)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(Clear, area);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

// ---------------------------------------------------------------------------
// Shared field renderer
// ---------------------------------------------------------------------------

/// Render a single `TextInput` line inside `area`.
/// When `focused` is true the label is highlighted yellow; otherwise dim gray.
pub fn render_text_field(frame: &mut Frame, area: Rect, input: &TextInput, focused: bool) {
    use ratatui::layout::{Constraint, Layout};

    let label_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let value_style = if focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };

    let cols = Layout::horizontal([Constraint::Length(14), Constraint::Min(0)]).areas::<2>(area);
    frame.render_widget(
        Paragraph::new(format!("{}: ", input.label)).style(label_style),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(format!("{}_", input.value)).style(value_style),
        cols[1],
    );
}

#[cfg(test)]
mod tests {
    use super::centered_rect;
    use ratatui::layout::Rect;

    #[test]
    fn popup_percentage_handles_large_terminal_dimensions() {
        assert_eq!(
            centered_rect(60, 70, Rect::new(0, 0, 2000, 1000)),
            Rect::new(400, 150, 1200, 700),
        );
        let area = Rect::new(5, 10, 81, 25);
        assert_eq!(centered_rect(60, 70, area), Rect::new(21, 14, 48, 17));
        assert_eq!(centered_rect(100, 100, area), area);
        assert_eq!(centered_rect(u16::MAX, u16::MAX, area), area);
    }
}
