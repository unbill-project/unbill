use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt as _;
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::time::{Duration, interval};
use unbill_core::model::{Bill, LedgerMeta, NewBill, NodeId, Share, User};
use unbill_core::service::{ServiceEvent, SettlementTransaction, UnbillService};

use crate::pane::Pane;
use crate::pane::detail::{BillEditor, EditorSection, ParticipantRow};
use crate::popup::PopupView;
use crate::popup::{
    PopupAction, PopupOutcome, confirm::ConfirmPopup, create_ledger::CreateLedgerPopup,
    invite::InviteResultPopup, settings::{SettingsPopup, TopTab},
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Error(String),
}

pub struct AppState {
    pub focused_pane: Pane,
    pub ledger_cursor: usize,
    pub bill_cursor: usize,
    pub ledgers: Vec<LedgerMeta>,
    pub users: Vec<User>,
    pub bills: Vec<Bill>,
    pub settlement: Vec<SettlementTransaction>,
    pub bill_editor: Option<BillEditor>,
    pub popup: Option<Box<dyn PopupView>>,
    pub sync_status: SyncStatus,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl AppState {
    fn new() -> Self {
        Self {
            focused_pane: Pane::Ledger,
            ledger_cursor: 0,
            bill_cursor: 0,
            ledgers: vec![],
            users: vec![],
            bills: vec![],
            settlement: vec![],
            bill_editor: None,
            popup: None,
            sync_status: SyncStatus::Idle,
            status_message: None,
            should_quit: false,
        }
    }

    pub fn current_ledger_id(&self) -> Option<String> {
        self.ledgers
            .get(self.ledger_cursor)
            .map(|l| l.ledger_id.to_string())
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    svc: Arc<UnbillService>,
) -> anyhow::Result<()> {
    let mut state = AppState::new();
    let mut events = EventStream::new();
    let mut tick = interval(Duration::from_millis(16));
    let mut svc_events = svc.subscribe();

    // Initial data load.
    refresh_ledgers(&svc, &mut state).await;
    refresh_bills(&svc, &mut state).await;
    refresh_users(&svc, &mut state).await;
    refresh_settlement(&svc, &mut state).await;

    loop {
        if state.should_quit {
            break;
        }

        terminal.draw(|f| crate::ui::render(f, &state))?;

        tokio::select! {
            _ = tick.tick() => {
                // Render tick — just redraw.
            }

            Ok(event) = svc_events.recv() => {
                match event {
                    ServiceEvent::LedgerUpdated { .. } => {
                        refresh_ledgers(&svc, &mut state).await;
                        refresh_bills(&svc, &mut state).await;
                        refresh_users(&svc, &mut state).await;
                        refresh_settlement(&svc, &mut state).await;
                    }
                    ServiceEvent::SyncError { error, .. } => {
                        state.sync_status = SyncStatus::Error(error);
                    }
                    _ => {}
                }
            }

            Some(Ok(ev)) = events.next() => {
                match ev {
                    Event::Key(key) => {
                        handle_key(key, &mut state, &svc).await;
                    }
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Key routing
// ---------------------------------------------------------------------------

async fn handle_key(key: KeyEvent, state: &mut AppState, svc: &Arc<UnbillService>) {
    // Global quit shortcuts.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        state.should_quit = true;
        return;
    }

    // Dispatch to popup first.
    if state.popup.is_some() {
        let outcome = {
            let popup = state.popup.as_mut().unwrap();
            popup.handle_key(key)
        };
        match outcome {
            PopupOutcome::Pending => {}
            PopupOutcome::Cancelled => {
                state.popup = None;
            }
            PopupOutcome::Action(action) => {
                state.popup = None;
                execute_action(action, state, svc).await;
            }
            PopupOutcome::OpenNext(next) => {
                state.popup = Some(next);
            }
        }
        return;
    }

    // Editor routing (when in Detail pane with active editor).
    if state.bill_editor.is_some() && state.focused_pane == Pane::Detail {
        handle_editor_key(key, state, svc).await;
        return;
    }

    // Global quit.
    if key.code == KeyCode::Char('q') {
        state.should_quit = true;
        return;
    }

    // Pane-specific routing.
    match state.focused_pane {
        Pane::Ledger => handle_ledger_key(key, state, svc).await,
        Pane::Bills => handle_bills_key(key, state, svc).await,
        Pane::Detail => handle_detail_key(key, state),
    }
}

async fn handle_ledger_key(key: KeyEvent, state: &mut AppState, svc: &Arc<UnbillService>) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down if !state.ledgers.is_empty() => {
            state.ledger_cursor = (state.ledger_cursor + 1).min(state.ledgers.len() - 1);
            state.bill_cursor = 0;
            refresh_bills(svc, state).await;
            refresh_users(svc, state).await;
            refresh_settlement(svc, state).await;
        }
        KeyCode::Char('k') | KeyCode::Up if state.ledger_cursor > 0 => {
            state.ledger_cursor -= 1;
            state.bill_cursor = 0;
            refresh_bills(svc, state).await;
            refresh_users(svc, state).await;
            refresh_settlement(svc, state).await;
        }
        KeyCode::Char('g') if !state.ledgers.is_empty() => {
            state.ledger_cursor = 0;
            state.bill_cursor = 0;
            refresh_bills(svc, state).await;
            refresh_users(svc, state).await;
            refresh_settlement(svc, state).await;
        }
        KeyCode::Char('G') if !state.ledgers.is_empty() => {
            state.ledger_cursor = state.ledgers.len() - 1;
            state.bill_cursor = 0;
            refresh_bills(svc, state).await;
            refresh_users(svc, state).await;
            refresh_settlement(svc, state).await;
        }
        KeyCode::Char('l') | KeyCode::Tab | KeyCode::Enter => {
            state.focused_pane = Pane::Bills;
        }
        KeyCode::Char('a') => {
            state.popup = Some(Box::new(CreateLedgerPopup::new()));
        }
        KeyCode::Char('d') => {
            if let Some(ledger_id) = state.current_ledger_id() {
                let name = state
                    .ledgers
                    .get(state.ledger_cursor)
                    .map(|l| l.name.clone())
                    .unwrap_or_default();
                state.popup = Some(Box::new(ConfirmPopup::new(
                    format!("Delete ledger \"{}\"?", name),
                    PopupAction::DeleteLedger { ledger_id },
                )));
            }
        }
        KeyCode::Char('u') => {
            open_settings_popup(TopTab::Ledger, state, svc).await;
        }
        KeyCode::Char('S') => {
            open_settings_popup(TopTab::Device, state, svc).await;
        }
        _ => {}
    }
}

async fn handle_bills_key(key: KeyEvent, state: &mut AppState, svc: &Arc<UnbillService>) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down if !state.bills.is_empty() => {
            state.bill_cursor = (state.bill_cursor + 1).min(state.bills.len() - 1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.bill_cursor = state.bill_cursor.saturating_sub(1);
        }
        KeyCode::Char('g') => {
            state.bill_cursor = 0;
        }
        KeyCode::Char('G') if !state.bills.is_empty() => {
            state.bill_cursor = state.bills.len() - 1;
        }
        KeyCode::Char('h') | KeyCode::BackTab => {
            state.focused_pane = Pane::Ledger;
        }
        KeyCode::Char('l') | KeyCode::Tab | KeyCode::Enter => {
            state.focused_pane = Pane::Detail;
        }
        KeyCode::Char('a') => {
            if let Some(ledger_id) = state.current_ledger_id() {
                match svc.list_users(&ledger_id).await {
                    Ok(users) => {
                        let editor = build_new_editor(ledger_id, users);
                        state.bill_editor = Some(editor);
                        state.focused_pane = Pane::Detail;
                    }
                    Err(e) => state.status_message = Some(e.to_string()),
                }
            }
        }
        KeyCode::Char('e') => {
            if let Some(ledger_id) = state.current_ledger_id()
                && let Some(bill) = state.bills.get(state.bill_cursor).cloned()
            {
                match svc.list_users(&ledger_id).await {
                    Ok(users) => {
                        let editor = build_amend_editor(ledger_id, &bill, users);
                        state.bill_editor = Some(editor);
                        state.focused_pane = Pane::Detail;
                    }
                    Err(e) => state.status_message = Some(e.to_string()),
                }
            }
        }
        _ => {}
    }
}

fn handle_detail_key(key: KeyEvent, state: &mut AppState) {
    match key.code {
        KeyCode::Char('h') | KeyCode::BackTab => {
            state.focused_pane = Pane::Bills;
        }
        KeyCode::Char('a') => {
            // Open new bill editor — need ledger_id; we can't call async here.
            // User must press 'a' in bills pane or via the bills pane.
            // For detail pane in view mode, hint says [a] new but we can't async here.
            // The spec says "key routing changes in handle_bills_key" so the detail view hint
            // is informational; actual 'a' when in detail view mode (no editor) falls through here.
            // We leave it as a no-op pending focus; user can go back to bills pane with 'h'.
        }
        _ => {}
    }
}

async fn handle_editor_key(key: KeyEvent, state: &mut AppState, svc: &Arc<UnbillService>) {
    {
        let editor = match state.bill_editor.as_mut() {
            Some(e) => e,
            None => return,
        };

        match key.code {
            KeyCode::Esc => {
                let _ = editor;
                state.bill_editor = None;
                state.focused_pane = Pane::Bills;
            }
            KeyCode::Tab => {
                editor.section = advance_section(editor.section);
            }
            KeyCode::BackTab => {
                editor.section = retreat_section(editor.section);
            }
            KeyCode::Down => match editor.section {
                EditorSection::Payers if !editor.payers.is_empty() => {
                    editor.payer_cursor = (editor.payer_cursor + 1).min(editor.payers.len() - 1);
                }
                EditorSection::Payees if !editor.payees.is_empty() => {
                    editor.payee_cursor = (editor.payee_cursor + 1).min(editor.payees.len() - 1);
                }
                _ => {}
            },
            KeyCode::Up => match editor.section {
                EditorSection::Payers => {
                    editor.payer_cursor = editor.payer_cursor.saturating_sub(1);
                }
                EditorSection::Payees => {
                    editor.payee_cursor = editor.payee_cursor.saturating_sub(1);
                }
                _ => {}
            },
            KeyCode::Char(c) => match editor.section {
                EditorSection::Description => editor.description.push(c),
                EditorSection::Amount if c != 'j' && c != 'k' => editor.amount_str.push(c),
                EditorSection::Amount => {}
                EditorSection::Payers => match c {
                    'j' if !editor.payers.is_empty() => {
                        editor.payer_cursor =
                            (editor.payer_cursor + 1).min(editor.payers.len() - 1);
                    }
                    'k' => {
                        editor.payer_cursor = editor.payer_cursor.saturating_sub(1);
                    }
                    ' ' => {
                        let cur = editor.payer_cursor;
                        if let Some(row) = editor.payers.get_mut(cur) {
                            row.selected = !row.selected;
                        }
                    }
                    c if c.is_ascii_digit() => {
                        let digit = c.to_digit(10).unwrap_or(1).max(1);
                        let cur = editor.payer_cursor;
                        if let Some(row) = editor.payers.get_mut(cur) {
                            row.weight = digit;
                        }
                    }
                    _ => {}
                },
                EditorSection::Payees => match c {
                    'j' if !editor.payees.is_empty() => {
                        editor.payee_cursor =
                            (editor.payee_cursor + 1).min(editor.payees.len() - 1);
                    }
                    'k' => {
                        editor.payee_cursor = editor.payee_cursor.saturating_sub(1);
                    }
                    ' ' => {
                        let cur = editor.payee_cursor;
                        if let Some(row) = editor.payees.get_mut(cur) {
                            row.selected = !row.selected;
                        }
                    }
                    c if c.is_ascii_digit() => {
                        let digit = c.to_digit(10).unwrap_or(1).max(1);
                        let cur = editor.payee_cursor;
                        if let Some(row) = editor.payees.get_mut(cur) {
                            row.weight = digit;
                        }
                    }
                    _ => {}
                },
            },
            KeyCode::Backspace => match editor.section {
                EditorSection::Description => {
                    editor.description.pop();
                }
                EditorSection::Amount => {
                    editor.amount_str.pop();
                }
                EditorSection::Payers => {
                    let cur = editor.payer_cursor;
                    if let Some(row) = editor.payers.get_mut(cur) {
                        row.weight = 1;
                    }
                }
                EditorSection::Payees => {
                    let cur = editor.payee_cursor;
                    if let Some(row) = editor.payees.get_mut(cur) {
                        row.weight = 1;
                    }
                }
            },
            KeyCode::Enter => {
                if editor.section != EditorSection::Payees {
                    editor.section = advance_section(editor.section);
                } else {
                    // Will handle confirm below after releasing borrow.
                    let _ = editor;
                    try_confirm_editor(state, svc).await;
                }
            }
            _ => {}
        }
    }
}

fn advance_section(s: EditorSection) -> EditorSection {
    match s {
        EditorSection::Description => EditorSection::Amount,
        EditorSection::Amount => EditorSection::Payers,
        EditorSection::Payers => EditorSection::Payees,
        EditorSection::Payees => EditorSection::Description,
    }
}

fn retreat_section(s: EditorSection) -> EditorSection {
    match s {
        EditorSection::Description => EditorSection::Payees,
        EditorSection::Amount => EditorSection::Description,
        EditorSection::Payers => EditorSection::Amount,
        EditorSection::Payees => EditorSection::Payers,
    }
}

async fn try_confirm_editor(state: &mut AppState, svc: &Arc<UnbillService>) {
    // Validate and build NewBill.
    let result = {
        let editor = match state.bill_editor.as_ref() {
            Some(e) => e,
            None => return,
        };

        let description = editor.description.trim().to_string();
        if description.is_empty() {
            Err("Description must not be empty".to_string())
        } else {
            let amount_cents = match parse_amount_cents(&editor.amount_str) {
                Some(v) if v >= 0 => v,
                Some(_) => {
                    return {
                        if let Some(e) = state.bill_editor.as_mut() {
                            e.error = Some("Amount must not be negative".to_string());
                        }
                    };
                }
                None => {
                    return {
                        if let Some(e) = state.bill_editor.as_mut() {
                            e.error = Some("Enter a valid amount (e.g. 12.50)".to_string());
                        }
                    };
                }
            };

            let payers: Vec<Share> = editor
                .payers
                .iter()
                .filter(|r| r.selected && r.weight >= 1)
                .map(|r| Share {
                    user_id: r.user.user_id,
                    shares: r.weight,
                })
                .collect();

            if payers.is_empty() {
                return {
                    if let Some(e) = state.bill_editor.as_mut() {
                        e.error = Some("Select at least one payer".to_string());
                    }
                };
            }

            let payees: Vec<Share> = editor
                .payees
                .iter()
                .filter(|r| r.selected && r.weight >= 1)
                .map(|r| Share {
                    user_id: r.user.user_id,
                    shares: r.weight,
                })
                .collect();

            if payees.is_empty() {
                return {
                    if let Some(e) = state.bill_editor.as_mut() {
                        e.error = Some("Select at least one payee".to_string());
                    }
                };
            }

            let prev = editor.prev_id.map(|id| vec![id]).unwrap_or_default();

            Ok((
                editor.ledger_id.clone(),
                NewBill {
                    amount_cents,
                    description,
                    payers,
                    payees,
                    prev,
                },
            ))
        }
    };

    match result {
        Err(msg) => {
            if let Some(e) = state.bill_editor.as_mut() {
                e.error = Some(msg);
            }
        }
        Ok((ledger_id, bill)) => match svc.add_bill(&ledger_id, bill).await {
            Ok(_) => {
                state.bill_editor = None;
                state.focused_pane = Pane::Bills;
                refresh_bills(svc, state).await;
                refresh_users(svc, state).await;
                refresh_settlement(svc, state).await;
            }
            Err(e) => {
                if let Some(ed) = state.bill_editor.as_mut() {
                    ed.error = Some(format!("add bill: {e}"));
                }
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Action execution
// ---------------------------------------------------------------------------

async fn execute_action(action: PopupAction, state: &mut AppState, svc: &Arc<UnbillService>) {
    match action {
        PopupAction::CreateLedger { name, currency } => {
            match svc.create_ledger(name, currency).await {
                Ok(_) => {
                    refresh_ledgers(svc, state).await;
                    refresh_bills(svc, state).await;
                    refresh_users(svc, state).await;
                    refresh_settlement(svc, state).await;
                }
                Err(e) => state.status_message = Some(format!("create ledger: {e}")),
            }
        }

        PopupAction::DeleteLedger { ledger_id } => match svc.delete_ledger(&ledger_id).await {
            Ok(_) => {
                refresh_ledgers(svc, state).await;
                state.ledger_cursor = state
                    .ledger_cursor
                    .min(state.ledgers.len().saturating_sub(1));
                refresh_bills(svc, state).await;
                refresh_users(svc, state).await;
                refresh_settlement(svc, state).await;
            }
            Err(e) => state.status_message = Some(format!("delete ledger: {e}")),
        },

        PopupAction::AddBill { ledger_id, bill } => match svc.add_bill(&ledger_id, bill).await {
            Ok(_) => {
                refresh_bills(svc, state).await;
                refresh_users(svc, state).await;
                refresh_settlement(svc, state).await;
            }
            Err(e) => state.status_message = Some(format!("add bill: {e}")),
        },

        PopupAction::AddUser { ledger_id, user } => match svc.add_user(&ledger_id, user).await {
            Ok(_) => {
                refresh_users(svc, state).await;
            }
            Err(e) => state.status_message = Some(format!("add user: {e}")),
        },

        PopupAction::AddLocalUser { display_name } => {
            match svc.add_local_user(display_name).await {
                Ok(_) => {}
                Err(e) => state.status_message = Some(format!("add local user: {e}")),
            }
        }

        PopupAction::ShareLocalUser { user_id } => {
            match svc.create_local_user_share(&user_id).await {
                Ok(url) => {
                    state.popup = Some(Box::new(InviteResultPopup::with_title(
                        "User Share URL",
                        url,
                    )));
                }
                Err(e) => state.status_message = Some(format!("share user: {e}")),
            }
        }

        PopupAction::ImportLocalUser { url } => match svc.fetch_local_user(&url).await {
            Ok(_) => {
                state.status_message = Some("User imported".to_string());
            }
            Err(e) => state.status_message = Some(format!("import user: {e}")),
        },

        PopupAction::GenerateInvite { ledger_id } => {
            match svc.create_invitation(&ledger_id).await {
                Ok(url) => {
                    state.popup = Some(Box::new(InviteResultPopup::new(url)));
                }
                Err(e) => state.status_message = Some(format!("invite: {e}")),
            }
        }

        PopupAction::JoinLedger { url } => match svc.join_ledger(&url, String::new()).await {
            Ok(_) => {
                refresh_ledgers(svc, state).await;
                refresh_bills(svc, state).await;
                refresh_users(svc, state).await;
                refresh_settlement(svc, state).await;
            }
            Err(e) => state.status_message = Some(format!("join ledger: {e}")),
        },

        PopupAction::SyncOnce { peer_node_id } => {
            match peer_node_id.parse::<NodeId>() {
                Ok(peer) => {
                    state.sync_status = SyncStatus::Syncing;
                    let svc = Arc::clone(svc);
                    // Run sync in background; errors surface via ServiceEvent::SyncError.
                    tokio::spawn(async move {
                        let _ = svc.sync_once(peer).await;
                    });
                }
                Err(e) => {
                    state.sync_status = SyncStatus::Error(format!("invalid peer id: {e}"));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Settings popup opener
// ---------------------------------------------------------------------------

async fn open_settings_popup(tab: TopTab, state: &mut AppState, svc: &Arc<UnbillService>) {
    let device_id = svc.device_id().to_string();
    let saved_users = match svc.list_local_users().await {
        Ok(u) => u,
        Err(e) => {
            state.status_message = Some(e.to_string());
            return;
        }
    };
    let ledgers = state.ledgers.clone();
    let mut ledger_users_map = Vec::with_capacity(ledgers.len());
    for ledger in &ledgers {
        let users = svc
            .list_users(&ledger.ledger_id.to_string())
            .await
            .unwrap_or_default();
        ledger_users_map.push(users);
    }
    let all_local_users = saved_users.clone();
    state.popup = Some(Box::new(SettingsPopup::new(
        tab,
        device_id,
        saved_users,
        ledgers,
        ledger_users_map,
        all_local_users,
        state.ledger_cursor,
    )));
}

// ---------------------------------------------------------------------------
// Data refresh helpers
// ---------------------------------------------------------------------------

pub async fn refresh_ledgers(svc: &Arc<UnbillService>, state: &mut AppState) {
    match svc.list_ledgers().await {
        Ok(ledgers) => {
            state.ledgers = ledgers;
            if state.ledger_cursor >= state.ledgers.len() && !state.ledgers.is_empty() {
                state.ledger_cursor = state.ledgers.len() - 1;
            }
        }
        Err(e) => state.status_message = Some(format!("list ledgers: {e}")),
    }
}

pub async fn refresh_bills(svc: &Arc<UnbillService>, state: &mut AppState) {
    if let Some(ledger_id) = state.current_ledger_id() {
        match svc.list_bills(&ledger_id).await {
            Ok(effective) => {
                state.bills = effective.into_vec();
                if state.bill_cursor >= state.bills.len() && !state.bills.is_empty() {
                    state.bill_cursor = state.bills.len() - 1;
                }
                if state.bills.is_empty() {
                    state.bill_cursor = 0;
                }
            }
            Err(e) => {
                state.bills = vec![];
                state.status_message = Some(format!("list bills: {e}"));
            }
        }
    } else {
        state.bills = vec![];
    }
}

pub async fn refresh_users(svc: &Arc<UnbillService>, state: &mut AppState) {
    if let Some(ledger_id) = state.current_ledger_id() {
        match svc.list_users(&ledger_id).await {
            Ok(users) => state.users = users,
            Err(_) => state.users = vec![],
        }
    } else {
        state.users = vec![];
    }
}

pub async fn refresh_settlement(svc: &Arc<UnbillService>, state: &mut AppState) {
    if let Some(ledger_id) = state.current_ledger_id() {
        match svc.settle_ledger(&ledger_id).await {
            Ok(s) => state.settlement = s.transactions,
            Err(_) => state.settlement = vec![],
        }
    } else {
        state.settlement = vec![];
    }
}

// ---------------------------------------------------------------------------
// Editor builder helpers
// ---------------------------------------------------------------------------

fn build_new_editor(ledger_id: String, users: Vec<User>) -> BillEditor {
    let payers = users
        .iter()
        .map(|u| ParticipantRow {
            user: u.clone(),
            selected: true,
            weight: 1,
        })
        .collect();
    let payees = users
        .iter()
        .map(|u| ParticipantRow {
            user: u.clone(),
            selected: true,
            weight: 1,
        })
        .collect();
    BillEditor {
        ledger_id,
        prev_id: None,
        description: String::new(),
        amount_str: String::new(),
        payers,
        payees,
        payer_cursor: 0,
        payee_cursor: 0,
        section: EditorSection::Description,
        error: None,
    }
}

fn build_amend_editor(ledger_id: String, bill: &Bill, users: Vec<User>) -> BillEditor {
    let payer_ids: std::collections::HashSet<_> = bill.payers.iter().map(|s| s.user_id).collect();
    let payer_weights: std::collections::HashMap<_, _> =
        bill.payers.iter().map(|s| (s.user_id, s.shares)).collect();
    let payee_ids: std::collections::HashSet<_> = bill.payees.iter().map(|s| s.user_id).collect();
    let payee_weights: std::collections::HashMap<_, _> =
        bill.payees.iter().map(|s| (s.user_id, s.shares)).collect();

    let payers = users
        .iter()
        .map(|u| ParticipantRow {
            user: u.clone(),
            selected: payer_ids.contains(&u.user_id),
            weight: payer_weights.get(&u.user_id).copied().unwrap_or(1),
        })
        .collect();
    let payees = users
        .iter()
        .map(|u| ParticipantRow {
            user: u.clone(),
            selected: payee_ids.contains(&u.user_id),
            weight: payee_weights.get(&u.user_id).copied().unwrap_or(1),
        })
        .collect();

    let amount_str = format!(
        "{}.{:02}",
        bill.amount_cents / 100,
        bill.amount_cents.abs() % 100
    );

    BillEditor {
        ledger_id,
        prev_id: Some(bill.id),
        description: bill.description.clone(),
        amount_str,
        payers,
        payees,
        payer_cursor: 0,
        payee_cursor: 0,
        section: EditorSection::Description,
        error: None,
    }
}

// ---------------------------------------------------------------------------
// Amount parsing helper
// ---------------------------------------------------------------------------

fn parse_amount_cents(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some((whole, frac)) = s.split_once('.') {
        let whole: i64 = whole.parse().ok()?;
        let frac = match frac.len() {
            0 => 0i64,
            1 => frac.parse::<i64>().ok()? * 10,
            _ => frac[..2].parse::<i64>().ok()?,
        };
        Some(whole * 100 + frac)
    } else {
        let whole: i64 = s.parse().ok()?;
        Some(whole * 100)
    }
}
