pub mod bills;
pub mod detail;
pub mod ledger;

/// The three panes of the TUI layout.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Ledgers,
    Bills,
    Detail,
}

impl Pane {
    /// Returns the key-hint string shown in the status bar for this pane.
    pub fn hints(&self) -> &'static str {
        match self {
            Pane::Ledgers => {
                "[j/k] move  [g/G] first/last  [l/Tab] pane  [a] create  [d] delete  [S] device  [q] quit"
            }
            Pane::Bills => {
                "[j/k] move  [g/G] first/last  [h/Shift+Tab] pane  [l/Tab] pane  [a] add bill  [e] amend  [u] settings  [q] quit"
            }
            Pane::Detail => "[h/Shift+Tab] back  [e] amend  [a] new  [q] quit",
        }
    }
}
