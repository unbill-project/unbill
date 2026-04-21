pub mod bills;
pub mod ledger;

/// The three panes of the TUI layout.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Ledger,
    Bills,
    Detail,
}

impl Pane {
    /// Returns the key-hint string shown in the status bar for this pane.
    pub fn hints(&self) -> &'static str {
        match self {
            Pane::Ledger => "[j/k] move  [g/G] first/last  [l/Tab] pane  [a] create  [d] delete  [u] users  [s] settle  [S] device  [i] invite  [q] quit",
            Pane::Bills => "[j/k] move  [g/G] first/last  [h/Shift+Tab] pane  [l/Tab] pane  [a] add bill  [e] amend  [q] quit",
            Pane::Detail => "[h/Shift+Tab] back  [q] quit",
        }
    }
}
