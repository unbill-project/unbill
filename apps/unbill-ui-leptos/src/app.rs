use crate::api::{
    self, AddLocalUserInput, AddUserInput, AppBootstrap, Bill, BillShareInput, JoinLedgerInput,
    LedgerDetail, LedgerSummary, SaveBillInput, User,
};
use crate::pages::{
    AddLocalUserSheet, AddUserSheet, BillEditorPage, CreateLedgerSheet, DeviceSettingsPage,
    EmptyColumn, JoinLedgerSheet, LedgerPage, LedgerSettingsPage, LedgersPage, StatusStrip,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

const COMPACT_BREAKPOINT: f64 = 1080.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SurfaceMode {
    Compact,
    Ranger,
}

#[derive(Clone, PartialEq)]
pub(crate) enum OverlayKind {
    CreateLedger,
    AddLocalUser,
    JoinLedger { url: String },
    AddUser,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShareMode {
    Equal,
    Custom,
}

#[derive(Clone, PartialEq)]
pub(crate) struct BillShareDraft {
    pub(crate) user_id: String,
    pub(crate) display_name: String,
    pub(crate) included: bool,
    pub(crate) shares: u32,
}

#[derive(Clone, PartialEq)]
pub(crate) struct BillEditorSeed {
    pub(crate) prev_bill_id: Option<String>,
    pub(crate) description: String,
    pub(crate) payer_mode: ShareMode,
    pub(crate) payer_rows: Vec<BillShareDraft>,
    pub(crate) amount_text: String,
    pub(crate) share_mode: ShareMode,
    pub(crate) share_rows: Vec<BillShareDraft>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct BillSaveRequest {
    pub(crate) prev_bill_id: Option<String>,
    pub(crate) description: String,
    pub(crate) payers: Vec<BillShareInput>,
    pub(crate) amount_cents: i64,
    pub(crate) shares: Vec<BillShareInput>,
}

#[component]
pub fn App() -> impl IntoView {
    let surface_mode = surface_mode_from_window();
    let bootstrap = RwSignal::new(None::<AppBootstrap>);
    let selected_ledger_id = RwSignal::new(None::<String>);
    let ledger_detail = RwSignal::new(None::<LedgerDetail>);
    let device_settings_open = RwSignal::new(false);
    let ledger_settings_open = RwSignal::new(false);
    let invitation_url = RwSignal::new(None::<String>);
    let overlay = RwSignal::new(None::<OverlayKind>);
    let bill_editor = RwSignal::new(None::<BillEditorSeed>);
    let status_message = RwSignal::new(None::<String>);
    let error_message = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    let load_selected_ledger = move |ledger_id: String| {
        selected_ledger_id.set(Some(ledger_id.clone()));
        busy.set(true);
        spawn_local(async move {
            match api::load_ledger_detail(&ledger_id).await {
                Ok(detail) => {
                    ledger_detail.set(Some(detail));
                    error_message.set(None);
                }
                Err(error) => error_message.set(Some(error)),
            }
            busy.set(false);
        });
    };

    let reload_bootstrap = move || {
        busy.set(true);
        spawn_local(async move {
            match api::bootstrap_app().await {
                Ok(mut data) => {
                    sort_ledgers(&mut data.ledgers);
                    let selected = selected_ledger_id.get_untracked();
                    let selection_exists = selected
                        .as_ref()
                        .map(|ledger_id| {
                            data.ledgers.iter().any(|item| &item.ledger_id == ledger_id)
                        })
                        .unwrap_or(false);

                    if !selection_exists {
                        selected_ledger_id.set(None);
                        ledger_detail.set(None);
                    }

                    if let Some(ledger_id) = selected
                        && selection_exists
                    {
                        match api::load_ledger_detail(&ledger_id).await {
                            Ok(detail) => ledger_detail.set(Some(detail)),
                            Err(error) => {
                                ledger_detail.set(None);
                                error_message.set(Some(error));
                            }
                        }
                    }

                    bootstrap.set(Some(data));
                    error_message.set(None);
                }
                Err(error) => error_message.set(Some(error)),
            }
            busy.set(false);
        });
    };

    reload_bootstrap();

    let open_ledger = move |ledger_id: String| {
        device_settings_open.set(false);
        ledger_settings_open.set(false);
        invitation_url.set(None);
        bill_editor.set(None);
        load_selected_ledger(ledger_id);
    };

    let open_new_bill = move || {
        if let Some(detail) = ledger_detail.get() {
            if detail.users.is_empty() {
                error_message.set(Some(
                    "Add at least one user to the ledger before creating a bill.".to_owned(),
                ));
                return;
            }
            bill_editor.set(Some(new_bill_seed(&detail.users)));
            error_message.set(None);
        }
    };

    let open_bill_amend = move |bill_id: String| {
        if let Some(detail) = ledger_detail.get()
            && let Some(bill) = detail.bills.iter().find(|item| item.id == bill_id)
        {
            bill_editor.set(Some(amend_bill_seed(bill, &detail.users)));
        }
    };

    let save_bill = move |request: BillSaveRequest| {
        if let Some(ledger_id) = selected_ledger_id.get() {
            busy.set(true);
            spawn_local(async move {
                match api::save_bill(SaveBillInput {
                    ledger_id: ledger_id.clone(),
                    description: request.description,
                    amount_cents: request.amount_cents,
                    payers: request.payers,
                    payees: request.shares,
                    prev_bill_id: request.prev_bill_id,
                })
                .await
                {
                    Ok(_) => {
                        bill_editor.set(None);
                        status_message.set(Some("Bill saved.".to_owned()));
                        error_message.set(None);
                        load_selected_ledger(ledger_id);
                        reload_bootstrap();
                    }
                    Err(error) => error_message.set(Some(error)),
                }
                busy.set(false);
            });
        }
    };

    let create_invitation = move || {
        if let Some(ledger_id) = selected_ledger_id.get() {
            busy.set(true);
            spawn_local(async move {
                match api::create_invitation(&ledger_id).await {
                    Ok(url) => {
                        invitation_url.set(Some(url));
                        status_message.set(Some("Invitation URL generated.".to_owned()));
                        error_message.set(None);
                    }
                    Err(error) => error_message.set(Some(error)),
                }
                busy.set(false);
            });
        }
    };

    let copy_invitation_url = move || {
        if let Some(url) = invitation_url.get() {
            spawn_local(async move {
                match api::write_clipboard_text(&url).await {
                    Ok(()) => {
                        status_message.set(Some("Invitation URL copied.".to_owned()));
                        error_message.set(None);
                    }
                    Err(error) => error_message.set(Some(error)),
                }
            });
        }
    };

    let open_join_from_clipboard = move || {
        spawn_local(async move {
            match api::read_clipboard_text().await {
                Ok(url) if !url.trim().is_empty() => {
                    overlay.set(Some(OverlayKind::JoinLedger { url }));
                    error_message.set(None);
                }
                Ok(_) => error_message.set(Some("Clipboard is empty.".to_owned())),
                Err(error) => error_message.set(Some(error)),
            }
        });
    };

    let sync_device = move |peer_node_id: String| {
        busy.set(true);
        spawn_local(async move {
            match api::sync_once(&peer_node_id).await {
                Ok(()) => {
                    status_message.set(Some("Sync completed.".to_owned()));
                    error_message.set(None);
                    if let Some(ledger_id) = selected_ledger_id.get_untracked() {
                        load_selected_ledger(ledger_id);
                    }
                    reload_bootstrap();
                }
                Err(error) => error_message.set(Some(error)),
            }
            busy.set(false);
        });
    };

    let render_overlay = move || {
        overlay.get().map(|sheet| match sheet {
            OverlayKind::CreateLedger => {
                view! {
                    <CreateLedgerSheet
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new(move |(name, currency): (String, String)| {
                            busy.set(true);
                            spawn_local(async move {
                                match api::create_ledger(api::CreateLedgerInput { name, currency }).await {
                                    Ok(summary) => {
                                        overlay.set(None);
                                        reload_bootstrap();
                                        open_ledger(summary.ledger_id);
                                        status_message.set(Some("Ledger created.".to_owned()));
                                        error_message.set(None);
                                    }
                                    Err(error) => error_message.set(Some(error)),
                                }
                                busy.set(false);
                            });
                        })
                    />
                }
                    .into_any()
            }
            OverlayKind::AddLocalUser => {
                view! {
                    <AddLocalUserSheet
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new(move |display_name: String| {
                            busy.set(true);
                            spawn_local(async move {
                                match api::add_local_user(AddLocalUserInput { display_name }).await {
                                    Ok(_) => {
                                        overlay.set(None);
                                        reload_bootstrap();
                                        status_message.set(Some("Saved user added on this device.".to_owned()));
                                        error_message.set(None);
                                    }
                                    Err(error) => error_message.set(Some(error)),
                                }
                                busy.set(false);
                            });
                        })
                    />
                }
                    .into_any()
            }
            OverlayKind::JoinLedger { url } => {
                view! {
                    <JoinLedgerSheet
                        initial_url=url
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new(move |(url, label): (String, String)| {
                            busy.set(true);
                            spawn_local(async move {
                                match api::join_ledger(JoinLedgerInput { url, label }).await {
                                    Ok(()) => {
                                        overlay.set(None);
                                        reload_bootstrap();
                                        status_message.set(Some("Ledger imported onto this device.".to_owned()));
                                        error_message.set(None);
                                    }
                                    Err(error) => error_message.set(Some(error)),
                                }
                                busy.set(false);
                            });
                        })
                    />
                }
                    .into_any()
            }
            OverlayKind::AddUser => {
                view! {
                    <AddUserSheet
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new(move |display_name: String| {
                            if let Some(ledger_id) = selected_ledger_id.get() {
                                busy.set(true);
                                spawn_local(async move {
                                    match api::add_user(AddUserInput { ledger_id: ledger_id.clone(), display_name }).await {
                                        Ok(_) => {
                                            overlay.set(None);
                                            load_selected_ledger(ledger_id);
                                            reload_bootstrap();
                                            status_message.set(Some("User added to ledger.".to_owned()));
                                            error_message.set(None);
                                        }
                                        Err(error) => error_message.set(Some(error)),
                                    }
                                    busy.set(false);
                                });
                            }
                        })
                    />
                }
                    .into_any()
            }
        })
    };

    let render_compact_page = move || {
        if let Some(seed) = bill_editor.get() {
            return view! {
                <div class="app-shell">
                    <BillEditorPage
                        title=if seed.prev_bill_id.is_some() {
                            "Amend Bill".to_owned()
                        } else {
                            "New Bill".to_owned()
                        }
                        currency=ledger_detail
                            .get()
                            .map(|detail| detail.summary.currency)
                            .unwrap_or_else(|| "USD".to_owned())
                        users=ledger_detail
                            .get()
                            .map(|detail| detail.users)
                            .unwrap_or_default()
                        seed=seed
                        on_back=Callback::new(move |_| bill_editor.set(None))
                        on_save=Callback::new(save_bill)
                    />
                </div>
            }
            .into_any();
        }

        if ledger_settings_open.get()
            && let Some(detail) = ledger_detail.get()
        {
            return view! {
                <div class="app-shell">
                    <LedgerSettingsPage
                        detail=detail
                        invitation_url=invitation_url.get()
                        on_back=Callback::new(move |_| ledger_settings_open.set(false))
                        on_add_user=Callback::new(move |_| overlay.set(Some(OverlayKind::AddUser)))
                        on_create_invitation=Callback::new(move |_| create_invitation())
                        on_copy_invitation=Callback::new(move |_| copy_invitation_url())
                    />
                </div>
            }
            .into_any();
        }

        if device_settings_open.get() {
            return view! {
                <div class="app-shell">
                    <DeviceSettingsPage
                        local_users=bootstrap.get().map(|data| data.local_users).unwrap_or_default()
                        devices=bootstrap.get().map(|data| data.devices).unwrap_or_default()
                        on_back=Callback::new(move |_| device_settings_open.set(false))
                        on_add_local_user=Callback::new(move |_| overlay.set(Some(OverlayKind::AddLocalUser)))
                        on_import_ledger=Callback::new(move |_| open_join_from_clipboard())
                        on_scan_qr=Callback::new(move |_| open_join_from_clipboard())
                        on_sync_device=Callback::new(sync_device)
                    />
                </div>
            }
            .into_any();
        }

        if let Some(detail) = ledger_detail.get() {
            return view! {
                <div class="app-shell">
                    <LedgerPage
                        detail=detail
                        on_back=Callback::new(move |_| {
                            selected_ledger_id.set(None);
                            ledger_detail.set(None);
                        })
                        on_more=Callback::new(move |_| {
                            ledger_settings_open.set(true);
                            invitation_url.set(None);
                        })
                        on_open_bill=Callback::new(open_bill_amend)
                        on_new_bill=Callback::new(move |_| open_new_bill())
                    />
                </div>
            }
            .into_any();
        }

        view! {
            <div class="app-shell">
                <LedgersPage
                    ledgers=bootstrap.get().map(|data| data.ledgers).unwrap_or_default()
                    selected_ledger_id=None
                    on_more=Callback::new(move |_| device_settings_open.set(true))
                    on_select_ledger=Callback::new(open_ledger)
                    on_new_ledger=Callback::new(move |_| overlay.set(Some(OverlayKind::CreateLedger)))
                />
            </div>
        }
        .into_any()
    };

    let render_ranger = move || {
        let ledgers = bootstrap.get().map(|data| data.ledgers).unwrap_or_default();
        let local_users = bootstrap
            .get()
            .map(|data| data.local_users)
            .unwrap_or_default();
        let selected_ledger = selected_ledger_id.get();

        let column_two = if device_settings_open.get() {
            view! {
                <DeviceSettingsPage
                    local_users=local_users
                    devices=bootstrap.get().map(|data| data.devices).unwrap_or_default()
                    on_back=Callback::new(move |_| device_settings_open.set(false))
                    on_add_local_user=Callback::new(move |_| overlay.set(Some(OverlayKind::AddLocalUser)))
                    on_import_ledger=Callback::new(move |_| open_join_from_clipboard())
                    on_scan_qr=Callback::new(move |_| open_join_from_clipboard())
                    on_sync_device=Callback::new(sync_device)
                />
            }
            .into_any()
        } else if let Some(detail) = ledger_detail.get() {
            view! {
                <LedgerPage
                    detail=detail
                    on_back=Callback::new(move |_| {
                        selected_ledger_id.set(None);
                        ledger_detail.set(None);
                    })
                    on_more=Callback::new(move |_| {
                        ledger_settings_open.set(true);
                        invitation_url.set(None);
                        bill_editor.set(None);
                    })
                    on_open_bill=Callback::new(open_bill_amend)
                    on_new_bill=Callback::new(move |_| open_new_bill())
                />
            }
            .into_any()
        } else {
            view! {
                <EmptyColumn
                    title="Select a ledger".to_owned()
                    detail="Choose a ledger to load bills and user state.".to_owned()
                />
            }
            .into_any()
        };

        let column_three = if let Some(seed) = bill_editor.get() {
            view! {
                <BillEditorPage
                    title=if seed.prev_bill_id.is_some() {
                        "Amend Bill".to_owned()
                    } else {
                        "New Bill".to_owned()
                    }
                    currency=ledger_detail
                        .get()
                        .map(|detail| detail.summary.currency)
                        .unwrap_or_else(|| "USD".to_owned())
                    users=ledger_detail
                        .get()
                        .map(|detail| detail.users)
                        .unwrap_or_default()
                    seed=seed
                    on_back=Callback::new(move |_| bill_editor.set(None))
                    on_save=Callback::new(save_bill)
                />
            }
            .into_any()
        } else if ledger_settings_open.get() {
            if let Some(detail) = ledger_detail.get() {
                view! {
                    <LedgerSettingsPage
                        detail=detail
                        invitation_url=invitation_url.get()
                        on_back=Callback::new(move |_| ledger_settings_open.set(false))
                        on_add_user=Callback::new(move |_| overlay.set(Some(OverlayKind::AddUser)))
                        on_create_invitation=Callback::new(move |_| create_invitation())
                        on_copy_invitation=Callback::new(move |_| copy_invitation_url())
                    />
                }
                .into_any()
            } else {
                view! {
                    <EmptyColumn
                        title="No ledger settings".to_owned()
                        detail="Select a ledger before opening settings.".to_owned()
                    />
                }
                .into_any()
            }
        } else {
            view! {
                <EmptyColumn
                    title="Select a bill".to_owned()
                    detail="Open a bill or ledger settings from the active ledger.".to_owned()
                />
            }
            .into_any()
        };

        view! {
            <main class="app-shell">
                <StatusStrip
                    status=status_message.get()
                    error=error_message.get()
                    busy=busy.get()
                />

                <section class="ranger-app-grid">
                    <LedgersPage
                        ledgers=ledgers
                        selected_ledger_id=selected_ledger
                        on_more=Callback::new(move |_| {
                            device_settings_open.set(true);
                            ledger_settings_open.set(false);
                            bill_editor.set(None);
                        })
                        on_select_ledger=Callback::new(open_ledger)
                        on_new_ledger=Callback::new(move |_| overlay.set(Some(OverlayKind::CreateLedger)))
                    />

                    {column_two}

                    {column_three}
                </section>

                {render_overlay()}
            </main>
        }
        .into_any()
    };

    view! {
        {move || {
            if surface_mode == SurfaceMode::Compact {
                view! {
                    <main class="app-shell">
                        <StatusStrip
                            status=status_message.get()
                            error=error_message.get()
                            busy=busy.get()
                        />
                        {render_compact_page()}
                        {render_overlay()}
                    </main>
                }
                    .into_any()
            } else {
                render_ranger()
            }
        }}
    }
}

pub(crate) fn surface_mode_from_window() -> SurfaceMode {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .map(|width| {
            if width < COMPACT_BREAKPOINT {
                SurfaceMode::Compact
            } else {
                SurfaceMode::Ranger
            }
        })
        .unwrap_or(SurfaceMode::Ranger)
}

fn sort_ledgers(ledgers: &mut [LedgerSummary]) {
    ledgers.sort_by(
        |left, right| match (left.latest_bill_at_ms, right.latest_bill_at_ms) {
            (Some(left_ts), Some(right_ts)) => right_ts
                .cmp(&left_ts)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase())),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        },
    );
}

fn new_bill_seed(users: &[User]) -> BillEditorSeed {
    BillEditorSeed {
        prev_bill_id: None,
        description: String::new(),
        payer_mode: ShareMode::Equal,
        payer_rows: users
            .iter()
            .enumerate()
            .map(|(i, user)| BillShareDraft {
                user_id: user.user_id.clone(),
                display_name: user.display_name.clone(),
                included: i == 0,
                shares: 1,
            })
            .collect(),
        amount_text: String::new(),
        share_mode: ShareMode::Equal,
        share_rows: users
            .iter()
            .map(|user| BillShareDraft {
                user_id: user.user_id.clone(),
                display_name: user.display_name.clone(),
                included: true,
                shares: 1,
            })
            .collect(),
    }
}

fn amend_bill_seed(bill: &Bill, users: &[User]) -> BillEditorSeed {
    let payers_by_user = bill
        .payers
        .iter()
        .map(|share| (share.user_id.clone(), share.shares))
        .collect::<std::collections::HashMap<_, _>>();
    let payees_by_user = bill
        .payees
        .iter()
        .map(|share| (share.user_id.clone(), share.shares))
        .collect::<std::collections::HashMap<_, _>>();

    let payer_mode = if payers_by_user.values().all(|&s| s == 1) {
        ShareMode::Equal
    } else {
        ShareMode::Custom
    };
    let share_mode = if payees_by_user.values().all(|&s| s == 1) {
        ShareMode::Equal
    } else {
        ShareMode::Custom
    };

    BillEditorSeed {
        prev_bill_id: Some(bill.id.clone()),
        description: bill.description.clone(),
        payer_mode,
        payer_rows: users
            .iter()
            .map(|user| BillShareDraft {
                user_id: user.user_id.clone(),
                display_name: user.display_name.clone(),
                included: payers_by_user.contains_key(&user.user_id),
                shares: payers_by_user.get(&user.user_id).copied().unwrap_or(1),
            })
            .collect(),
        amount_text: format!(
            "{}.{:02}",
            bill.amount_cents / 100,
            bill.amount_cents.abs() % 100
        ),
        share_mode,
        share_rows: users
            .iter()
            .map(|user| BillShareDraft {
                user_id: user.user_id.clone(),
                display_name: user.display_name.clone(),
                included: payees_by_user.contains_key(&user.user_id),
                shares: payees_by_user.get(&user.user_id).copied().unwrap_or(1),
            })
            .collect(),
    }
}

pub(crate) fn parse_amount_text(input: &str) -> Result<i64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter an amount before saving.".to_owned());
    }
    if trimmed.starts_with('-') {
        return Err("Amount must be zero or greater.".to_owned());
    }

    let mut parts = trimmed.split('.');
    let units = parts
        .next()
        .unwrap_or_default()
        .parse::<i64>()
        .map_err(|_| "Amount must use digits and an optional decimal point.".to_owned())?;
    let cents = match parts.next() {
        None => 0,
        Some(raw) => {
            if parts.next().is_some() {
                return Err("Amount can contain only one decimal point.".to_owned());
            }
            let padded = if raw.len() == 1 {
                format!("{raw}0")
            } else {
                raw.to_owned()
            };
            if padded.len() != 2 {
                return Err("Amount must use at most two decimal places.".to_owned());
            }
            padded
                .parse::<i64>()
                .map_err(|_| "Amount cents must be numeric.".to_owned())?
        }
    };

    Ok(units * 100 + cents)
}

pub(crate) fn share_lookup_shares(share_rows: &[BillShareDraft], user_id: &str) -> u32 {
    share_rows
        .iter()
        .find(|share_row| share_row.user_id == user_id)
        .map(|share_row| share_row.shares)
        .unwrap_or(1)
}

pub(crate) fn derived_share_preview(
    amount_cents: i64,
    share_mode: ShareMode,
    share_rows: &[BillShareDraft],
) -> Vec<(String, i64)> {
    let active = share_rows
        .iter()
        .filter(|share_row| share_row.included)
        .map(|share_row| {
            (
                share_row.user_id.clone(),
                if share_mode == ShareMode::Equal {
                    1
                } else {
                    share_row.shares
                },
            )
        })
        .collect::<Vec<_>>();

    let total_shares = active.iter().map(|(_, shares)| *shares as i64).sum::<i64>();
    if total_shares == 0 {
        return Vec::new();
    }

    let mut allocations = active
        .iter()
        .map(|(user_id, shares)| {
            (
                user_id.clone(),
                amount_cents * *shares as i64 / total_shares,
            )
        })
        .collect::<Vec<_>>();
    let assigned = allocations.iter().map(|(_, amount)| *amount).sum::<i64>();
    let mut remainder = amount_cents - assigned;
    for (_, amount) in allocations.iter_mut() {
        if remainder == 0 {
            break;
        }
        *amount += 1;
        remainder -= 1;
    }
    allocations
}
