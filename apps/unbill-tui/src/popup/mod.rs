use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Clear, Paragraph},
};

use unbill_core::model::{NewBill, NewUser};

pub mod confirm;
pub mod create_ledger;
pub mod device;
pub mod invite;
pub mod ledger_settings;

/// Trait implemented by every popup view.
pub trait PopupView: Send {
    fn title(&self) -> &str;
    fn render(&self, frame: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyEvent) -> PopupOutcome;
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
}

/// Describes the service mutation to perform after a popup confirms.
#[allow(dead_code)]
pub enum PopupAction {
    CreateLedger { name: String, currency: String },
    DeleteLedger { ledger_id: String },
    AddBill { ledger_id: String, bill: NewBill },
    AddUser { ledger_id: String, user: NewUser },
    AddLocalUser { display_name: String },
    ShareLocalUser { user_id: String },
    ImportLocalUser { url: String },
    GenerateInvite { ledger_id: String },
    JoinLedger { url: String },
    SyncOnce { peer_node_id: String },
}

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
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    use ratatui::layout::{Constraint, Layout};

    let popup_height = r.height * percent_y / 100;
    let popup_width = r.width * percent_x / 100;

    let vertical = Layout::vertical([
        Constraint::Length((r.height.saturating_sub(popup_height)) / 2),
        Constraint::Length(popup_height),
        Constraint::Min(0),
    ])
    .split(r);

    let horizontal = Layout::horizontal([
        Constraint::Length((r.width.saturating_sub(popup_width)) / 2),
        Constraint::Length(popup_width),
        Constraint::Min(0),
    ])
    .split(vertical[1]);

    horizontal[1]
}

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

    let cols = Layout::horizontal([Constraint::Length(14), Constraint::Min(0)]).split(area);
    frame.render_widget(
        Paragraph::new(format!("{}: ", input.label)).style(label_style),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(format!("{}_", input.value)).style(value_style),
        cols[1],
    );
}
