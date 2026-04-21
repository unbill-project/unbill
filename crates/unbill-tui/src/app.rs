use std::collections::HashMap;
use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt as _;
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::time::{Duration, interval};
use unbill_core::model::{Bill, LedgerMeta, NodeId};
use unbill_core::service::{ServiceEvent, UnbillService};

use crate::pane::Pane;
use crate::popup::PopupView;
use crate::popup::{
    PopupAction, PopupOutcome,
    confirm::ConfirmPopup,
    create_ledger::CreateLedgerPopup,
    device::DevicePopup,
    invite::{InvitePopup, InviteResultPopup},
    settlement::{PickUserPopup, SettlementResultPopup},
    users::UsersPopup,
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
    pub bills: Vec<Bill>,
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
            bills: vec![],
            popup: None,
            sync_status: SyncStatus::Idle,
            status_message: None,
            should_quit: false,
        }
    }

    fn current_ledger_id(&self) -> Option<String> {
        self.ledgers.get(self.ledger_cursor).map(|l| l.ledger_id.to_string())
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
        KeyCode::Char('j') | KeyCode::Down => {
            if !state.ledgers.is_empty() {
                state.ledger_cursor = (state.ledger_cursor + 1).min(state.ledgers.len() - 1);
                state.bill_cursor = 0;
                refresh_bills(svc, state).await;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.ledger_cursor > 0 {
                state.ledger_cursor -= 1;
                state.bill_cursor = 0;
                refresh_bills(svc, state).await;
            }
        }
        KeyCode::Char('g') => {
            if !state.ledgers.is_empty() {
                state.ledger_cursor = 0;
                state.bill_cursor = 0;
                refresh_bills(svc, state).await;
            }
        }
        KeyCode::Char('G') => {
            if !state.ledgers.is_empty() {
                state.ledger_cursor = state.ledgers.len() - 1;
                state.bill_cursor = 0;
                refresh_bills(svc, state).await;
            }
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
            if let Some(ledger_id) = state.current_ledger_id() {
                match svc.list_users(&ledger_id).await {
                    Ok(ledger_users) => match svc.list_local_users().await {
                        Ok(local_users) => {
                            state.popup = Some(Box::new(UsersPopup::new(
                                ledger_id,
                                ledger_users,
                                local_users,
                            )));
                        }
                        Err(e) => state.status_message = Some(e.to_string()),
                    },
                    Err(e) => state.status_message = Some(e.to_string()),
                }
            }
        }
        KeyCode::Char('s') => {
            match svc.list_local_users().await {
                Ok(local_users) => {
                    state.popup = Some(Box::new(PickUserPopup::new(local_users)));
                }
                Err(e) => state.status_message = Some(e.to_string()),
            }
        }
        KeyCode::Char('S') => {
            let device_id = svc.device_id().to_string();
            state.popup = Some(Box::new(DevicePopup::new(device_id)));
        }
        KeyCode::Char('i') => {
            if let Some(ledger_id) = state.current_ledger_id() {
                state.popup = Some(Box::new(InvitePopup::new(ledger_id)));
            }
        }
        _ => {}
    }
}

async fn handle_bills_key(key: KeyEvent, state: &mut AppState, svc: &Arc<UnbillService>) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if !state.bills.is_empty() {
                state.bill_cursor = (state.bill_cursor + 1).min(state.bills.len() - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.bill_cursor = state.bill_cursor.saturating_sub(1);
        }
        KeyCode::Char('g') => {
            state.bill_cursor = 0;
        }
        KeyCode::Char('G') => {
            if !state.bills.is_empty() {
                state.bill_cursor = state.bills.len() - 1;
            }
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
                        state.popup = Some(Box::new(
                            crate::popup::add_bill::AddBillPopup::new(ledger_id, users),
                        ));
                    }
                    Err(e) => state.status_message = Some(e.to_string()),
                }
            }
        }
        KeyCode::Char('e') => {
            if let Some(ledger_id) = state.current_ledger_id() {
                if let Some(bill) = state.bills.get(state.bill_cursor) {
                    let bill = bill.clone();
                    match svc.list_users(&ledger_id).await {
                        Ok(users) => {
                            state.popup = Some(Box::new(
                                crate::popup::amend_bill::AmendBillPopup::new(
                                    ledger_id, &bill, users,
                                ),
                            ));
                        }
                        Err(e) => state.status_message = Some(e.to_string()),
                    }
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
        _ => {}
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
                }
                Err(e) => state.status_message = Some(format!("create ledger: {e}")),
            }
        }

        PopupAction::DeleteLedger { ledger_id } => {
            match svc.delete_ledger(&ledger_id).await {
                Ok(_) => {
                    refresh_ledgers(svc, state).await;
                    state.ledger_cursor =
                        state.ledger_cursor.min(state.ledgers.len().saturating_sub(1));
                    refresh_bills(svc, state).await;
                }
                Err(e) => state.status_message = Some(format!("delete ledger: {e}")),
            }
        }

        PopupAction::AddBill { ledger_id, bill } => {
            match svc.add_bill(&ledger_id, bill).await {
                Ok(_) => {
                    refresh_bills(svc, state).await;
                }
                Err(e) => state.status_message = Some(format!("add bill: {e}")),
            }
        }

        PopupAction::AddUser { ledger_id, user } => {
            match svc.add_user(&ledger_id, user).await {
                Ok(_) => {}
                Err(e) => state.status_message = Some(format!("add user: {e}")),
            }
        }

        PopupAction::AddLocalUser { display_name } => {
            match svc.add_local_user(display_name).await {
                Ok(_) => {}
                Err(e) => state.status_message = Some(format!("add local user: {e}")),
            }
        }

        PopupAction::ShowSettlement { user_id, display_name } => {
            match svc.compute_settlement_for_user(&user_id).await {
                Ok(settlement) => {
                    // Build a name map from all known users across ledgers.
                    let mut user_names: HashMap<String, String> = HashMap::new();
                    if let Ok(ledgers) = svc.list_ledgers().await {
                        for meta in &ledgers {
                            let lid = meta.ledger_id.to_string();
                            if let Ok(users) = svc.list_users(&lid).await {
                                for u in users {
                                    user_names.insert(u.user_id.to_string(), u.display_name);
                                }
                            }
                        }
                    }
                    // Also add local users.
                    if let Ok(local_users) = svc.list_local_users().await {
                        for u in local_users {
                            user_names
                                .entry(u.user_id.to_string())
                                .or_insert(u.display_name);
                        }
                    }
                    state.popup = Some(Box::new(SettlementResultPopup::new(
                        display_name,
                        settlement.transactions,
                        user_names,
                    )));
                }
                Err(e) => state.status_message = Some(format!("settlement: {e}")),
            }
        }

        PopupAction::GenerateInvite { ledger_id } => {
            match svc.create_invitation(&ledger_id).await {
                Ok(url) => {
                    state.popup = Some(Box::new(InviteResultPopup::new(url)));
                }
                Err(e) => state.status_message = Some(format!("invite: {e}")),
            }
        }

        PopupAction::JoinLedger { url } => {
            match svc.join_ledger(&url, String::new()).await {
                Ok(_) => {
                    refresh_ledgers(svc, state).await;
                    refresh_bills(svc, state).await;
                }
                Err(e) => state.status_message = Some(format!("join ledger: {e}")),
            }
        }

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
