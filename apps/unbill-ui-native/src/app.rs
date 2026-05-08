use crate::api::{
    self, AddUserInput, Bill, BillShareInput, CreateUserInput, JoinLedgerInput, LedgerDetail,
    LedgerSummary, SaveBillInput, SyncDevice, User,
};
use crate::components::{EmptyColumn, StatusStrip};
use crate::pages::{
    AddLedgerUserSheet, BillEditorPage, CreateLedgerSheet, JoinLedgerSheet, LedgerPage,
    LedgersPage, SettingsPopup,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, closure::Closure};

const RANGER_BREAKPOINT: f64 = 1200.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceMode {
    Compact,
    Ranger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    Device,
    Ledger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SettingsPopupState {
    pub(crate) active_tab: SettingsTab,
    pub(crate) selected_ledger_id: Option<String>,
}

impl SettingsPopupState {
    pub(crate) fn open_device() -> Self {
        Self {
            active_tab: SettingsTab::Device,
            selected_ledger_id: None,
        }
    }

    pub(crate) fn open_ledger(active_ledger_id: Option<String>, ledgers: &[LedgerSummary]) -> Self {
        Self {
            active_tab: SettingsTab::Ledger,
            selected_ledger_id: active_ledger_id
                .or_else(|| ledgers.first().map(|ledger| ledger.ledger_id.clone())),
        }
    }

    pub(crate) fn select_ledger(&mut self, ledger_id: String) {
        self.selected_ledger_id = Some(ledger_id);
    }

    pub(crate) fn select_tab(&mut self, tab: SettingsTab) {
        self.active_tab = tab;
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RangerColumns {
    pub(crate) ledgers: bool,
    pub(crate) bills: bool,
    pub(crate) detail: bool,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RangerComposition {
    pub(crate) columns: RangerColumns,
    pub(crate) settings_overlay: bool,
}

#[cfg(test)]
pub(crate) fn ranger_composition(settings_open: bool) -> RangerComposition {
    RangerComposition {
        columns: RangerColumns {
            ledgers: true,
            bills: true,
            detail: true,
        },
        settings_overlay: settings_open,
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompactBaseSurface {
    Detail,
    Bills,
    Ledgers,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompactComposition {
    pub(crate) base_surface: CompactBaseSurface,
    pub(crate) settings_overlay: bool,
}

#[cfg(test)]
pub(crate) fn compact_composition(
    settings_open: bool,
    has_bill_editor: bool,
    has_ledger_detail: bool,
) -> CompactComposition {
    let base_surface = if has_bill_editor {
        CompactBaseSurface::Detail
    } else if has_ledger_detail {
        CompactBaseSurface::Bills
    } else {
        CompactBaseSurface::Ledgers
    };

    CompactComposition {
        base_surface,
        settings_overlay: settings_open,
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum OverlayKind {
    CreateLedger,
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
    pub(crate) currency: String,
    pub(crate) users: Vec<User>,
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
    let surface_mode = RwSignal::new(surface_mode_from_window());
    install_surface_mode_resize_listener(surface_mode);
    let device_id = RwSignal::new(String::new());
    let ledgers = RwSignal::new(Vec::<LedgerSummary>::new());
    let all_users = RwSignal::new(Vec::<User>::new());
    let devices = RwSignal::new(Vec::<SyncDevice>::new());
    let selected_ledger_id = RwSignal::new(None::<String>);
    let ledger_detail = RwSignal::new(None::<LedgerDetail>);
    let settings_popup = RwSignal::new(None::<SettingsPopupState>);
    let settings_ledger_detail = RwSignal::new(None::<LedgerDetail>);
    let invitation_url = RwSignal::new(None::<String>);
    let overlay = RwSignal::new(None::<OverlayKind>);
    let bill_editor = RwSignal::new(None::<BillEditorSeed>);
    let status_message = RwSignal::new(None::<String>);
    let error_message = RwSignal::new(None::<String>);
    let loading_count = RwSignal::new(0usize);

    let load_selected_ledger = move |ledger_id: String| {
        selected_ledger_id.set(Some(ledger_id.clone()));
        loading_count.update(|n| *n += 1);
        spawn_local(async move {
            match api::load_ledger_detail(&ledger_id).await {
                Ok(detail) => {
                    ledger_detail.set(Some(detail));
                    error_message.set(None);
                }
                Err(error) => error_message.set(Some(error)),
            }
            loading_count.update(|n| *n = n.saturating_sub(1));
        });
    };

    let load_settings_ledger = move |ledger_id: String| {
        loading_count.update(|n| *n += 1);
        spawn_local(async move {
            match api::load_ledger_detail(&ledger_id).await {
                Ok(detail) => {
                    settings_ledger_detail.set(Some(detail));
                    error_message.set(None);
                }
                Err(error) => {
                    settings_ledger_detail.set(None);
                    error_message.set(Some(error));
                }
            }
            loading_count.update(|n| *n = n.saturating_sub(1));
        });
    };

    let reload_bootstrap = move || {
        loading_count.update(|n| *n += 1);
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
                        && let Ok(detail) = api::load_ledger_detail(&ledger_id).await
                    {
                        ledger_detail.set(Some(detail));
                    }

                    if let Some(mut popup) = settings_popup.get_untracked() {
                        if let Some(settings_ledger_id) = popup.selected_ledger_id.clone() {
                            let settings_selection_exists = data
                                .ledgers
                                .iter()
                                .any(|item| item.ledger_id == settings_ledger_id);

                            if settings_selection_exists {
                                match api::load_ledger_detail(&settings_ledger_id).await {
                                    Ok(detail) => settings_ledger_detail.set(Some(detail)),
                                    Err(error) => {
                                        settings_ledger_detail.set(None);
                                        error_message.set(Some(error));
                                    }
                                }
                            } else {
                                popup.selected_ledger_id =
                                    data.ledgers.first().map(|ledger| ledger.ledger_id.clone());
                                settings_popup.set(Some(popup.clone()));

                                if let Some(next_id) = popup.selected_ledger_id {
                                    match api::load_ledger_detail(&next_id).await {
                                        Ok(detail) => settings_ledger_detail.set(Some(detail)),
                                        Err(error) => {
                                            settings_ledger_detail.set(None);
                                            error_message.set(Some(error));
                                        }
                                    }
                                } else {
                                    settings_ledger_detail.set(None);
                                }
                            }
                        } else {
                            settings_ledger_detail.set(None);
                        }
                    }

                    device_id.set(data.device_id);
                    ledgers.set(data.ledgers);
                    all_users.set(data.all_users);
                    devices.set(data.devices);
                    error_message.set(None);
                }
                Err(error) => error_message.set(Some(error)),
            }
            loading_count.update(|n| *n = n.saturating_sub(1));
        });
    };

    reload_bootstrap();

    let open_ledger = move |ledger_id: String| {
        settings_popup.set(None);
        settings_ledger_detail.set(None);
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
            bill_editor.set(Some(new_bill_seed(detail.summary.currency, &detail.users)));
            error_message.set(None);
        }
    };

    let open_bill_amend = move |bill_id: String| {
        if let Some(detail) = ledger_detail.get()
            && let Some(bill) = detail.bills.iter().find(|item| item.id == bill_id)
        {
            bill_editor.set(Some(amend_bill_seed(
                bill,
                detail.summary.currency,
                &detail.users,
            )));
        }
    };

    let save_bill = move |request: BillSaveRequest| {
        if let Some(ledger_id) = selected_ledger_id.get() {
            loading_count.update(|n| *n += 1);
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
                        if let Ok(detail) = api::load_ledger_detail(&ledger_id).await {
                            ledgers.update(|list| {
                                if let Some(entry) =
                                    list.iter_mut().find(|l| l.ledger_id == ledger_id)
                                {
                                    *entry = detail.summary.clone();
                                    sort_ledgers(list);
                                }
                            });
                            ledger_detail.set(Some(detail));
                        }
                    }
                    Err(error) => error_message.set(Some(error)),
                }
                loading_count.update(|n| *n = n.saturating_sub(1));
            });
        }
    };

    let create_invitation = move || {
        if let Some(ledger_id) = settings_popup
            .get()
            .and_then(|popup| popup.selected_ledger_id)
            .or_else(|| selected_ledger_id.get())
        {
            loading_count.update(|n| *n += 1);
            spawn_local(async move {
                match api::create_invitation(&ledger_id).await {
                    Ok(url) => {
                        invitation_url.set(Some(url));
                        status_message.set(Some("Invitation URL generated.".to_owned()));
                        error_message.set(None);
                    }
                    Err(error) => error_message.set(Some(error)),
                }
                loading_count.update(|n| *n = n.saturating_sub(1));
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

    let open_device_settings = move || {
        settings_popup.set(Some(SettingsPopupState::open_device()));
        settings_ledger_detail.set(None);
        invitation_url.set(None);
    };

    let open_ledger_settings = move || {
        let popup = SettingsPopupState::open_ledger(selected_ledger_id.get(), &ledgers.get());
        if let Some(ledger_id) = popup.selected_ledger_id.clone() {
            if ledger_detail
                .get()
                .as_ref()
                .map(|detail| detail.summary.ledger_id == ledger_id)
                .unwrap_or(false)
            {
                settings_ledger_detail.set(ledger_detail.get());
            } else {
                settings_ledger_detail.set(None);
                load_settings_ledger(ledger_id);
            }
        } else {
            settings_ledger_detail.set(None);
        }
        settings_popup.set(Some(popup));
        invitation_url.set(None);
    };

    let select_settings_tab = move |tab: SettingsTab| {
        settings_popup.update(|popup| {
            if let Some(popup) = popup {
                popup.select_tab(tab);
            }
        });
    };

    let select_settings_ledger = move |ledger_id: String| {
        settings_popup.update(|popup| {
            if let Some(popup) = popup {
                popup.select_ledger(ledger_id.clone());
            }
        });
        invitation_url.set(None);
        settings_ledger_detail.set(None);
        load_settings_ledger(ledger_id);
    };

    let sync_device = move |peer_node_id: String| {
        loading_count.update(|n| *n += 1);
        spawn_local(async move {
            match api::sync_once(&peer_node_id).await {
                Ok(()) => {
                    status_message.set(Some("Sync completed.".to_owned()));
                    error_message.set(None);
                    reload_bootstrap();
                }
                Err(error) => error_message.set(Some(error)),
            }
            loading_count.update(|n| *n = n.saturating_sub(1));
        });
    };

    let render_overlay = move || {
        overlay.get().map(|sheet| match sheet {
            OverlayKind::CreateLedger => {
                view! {
                    <CreateLedgerSheet
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new(move |(name, currency): (String, String)| {
                            loading_count.update(|n| *n += 1);
                            spawn_local(async move {
                                match api::create_ledger(api::CreateLedgerInput { name, currency }).await {
                                    Ok(summary) => {
                                        overlay.set(None);
                                        ledgers.update(|l| {
                                            l.push(summary.clone());
                                            sort_ledgers(l);
                                        });
                                        open_ledger(summary.ledger_id);
                                        status_message.set(Some("Ledger created.".to_owned()));
                                        error_message.set(None);
                                    }
                                    Err(error) => error_message.set(Some(error)),
                                }
                                loading_count.update(|n| *n = n.saturating_sub(1));
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
                            loading_count.update(|n| *n += 1);
                            spawn_local(async move {
                                match api::join_ledger(JoinLedgerInput { url, label }).await {
                                    Ok(()) => {
                                        overlay.set(None);
                                        reload_bootstrap();
                                        status_message.set(Some(
                                            "Ledger imported onto this device.".to_owned(),
                                        ));
                                        error_message.set(None);
                                    }
                                    Err(error) => error_message.set(Some(error)),
                                }
                                loading_count.update(|n| *n = n.saturating_sub(1));
                            });
                        })
                    />
                }
                .into_any()
            }
            OverlayKind::AddUser => {
                view! {
                    <AddLedgerUserSheet
                        all_users=all_users.get()
                        ledger_users=settings_ledger_detail.get().map(|detail| detail.users).unwrap_or_default()
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new(move |user_id: String| {
                            if let Some(ledger_id) = settings_popup
                                .get()
                                .and_then(|popup| popup.selected_ledger_id)
                            {
                                loading_count.update(|n| *n += 1);
                                spawn_local(async move {
                                    match api::add_user(AddUserInput {
                                        ledger_id: ledger_id.clone(),
                                        user_id,
                                    })
                                    .await
                                    {
                                        Ok(_) => {
                                            overlay.set(None);
                                            reload_bootstrap();
                                            status_message.set(Some(
                                                "User added to ledger.".to_owned(),
                                            ));
                                            error_message.set(None);
                                        }
                                        Err(error) => error_message.set(Some(error)),
                                    }
                                    loading_count.update(|n| *n = n.saturating_sub(1));
                                });
                            }
                        })
                        on_create_user=Callback::new(move |display_name: String| {
                            if let Some(ledger_id) = settings_popup
                                .get()
                                .and_then(|popup| popup.selected_ledger_id)
                            {
                                loading_count.update(|n| *n += 1);
                                spawn_local(async move {
                                    match api::create_user(CreateUserInput {
                                        ledger_id: ledger_id.clone(),
                                        display_name,
                                    })
                                    .await
                                    {
                                        Ok(_) => {
                                            overlay.set(None);
                                            reload_bootstrap();
                                            status_message.set(Some("User created.".to_owned()));
                                            error_message.set(None);
                                        }
                                        Err(error) => error_message.set(Some(error)),
                                    }
                                    loading_count.update(|n| *n = n.saturating_sub(1));
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
                <BillEditorPage
                    title=if seed.prev_bill_id.is_some() {
                        "Amend Bill".to_owned()
                    } else {
                        "New Bill".to_owned()
                    }
                    currency=seed.currency.clone()
                    users=seed.users.clone()
                    seed=seed
                    on_back=Callback::new(move |_| bill_editor.set(None))
                    on_save=Callback::new(save_bill)
                />
            }
            .into_any();
        }

        if let Some(detail) = ledger_detail.get() {
            return view! {
                <LedgerPage
                    detail=detail
                    on_back=Callback::new(move |_| {
                        selected_ledger_id.set(None);
                        ledger_detail.set(None);
                    })
                    on_more=Callback::new(move |_| open_ledger_settings())
                    on_open_bill=Callback::new(open_bill_amend)
                    on_new_bill=Callback::new(move |_| open_new_bill())
                />
            }
            .into_any();
        }

        view! {
            <LedgersPage
                ledgers=ledgers.get()
                selected_ledger_id=None
                on_more=Callback::new(move |_| open_device_settings())
                on_select_ledger=Callback::new(open_ledger)
                on_new_ledger=Callback::new(move |_| overlay.set(Some(OverlayKind::CreateLedger)))
            />
        }
        .into_any()
    };

    let render_settings_popup = move || {
        settings_popup.get().map(|popup| {
            view! {
                <SettingsPopup
                    device_id=device_id.get()
                    ledgers=ledgers.get()
                    devices=devices.get()
                    active_tab=popup.active_tab
                    selected_ledger_id=popup.selected_ledger_id
                    ledger_detail=settings_ledger_detail.get()
                    invitation_url=invitation_url.get()
                    on_close=Callback::new(move |_| {
                        settings_popup.set(None);
                        settings_ledger_detail.set(None);
                    })
                    on_select_tab=Callback::new(select_settings_tab)
                    on_select_ledger=Callback::new(select_settings_ledger)
                    on_join_ledger=Callback::new(move |_| open_join_from_clipboard())
                    on_add_ledger_user=Callback::new(move |_| overlay.set(Some(OverlayKind::AddUser)))
                    on_sync_device=Callback::new(sync_device)
                    on_create_invitation=Callback::new(move |_| create_invitation())
                    on_copy_invitation=Callback::new(move |_| copy_invitation_url())
                />
            }
        })
    };

    let render_ranger = move || {
        view! {
            <main class="app-shell">
                {move || view! {
                    <StatusStrip
                        status=status_message.get()
                        error=error_message.get()
                        busy={ loading_count.get() != 0 }
                    />
                }}

                <div class="safe-content-area">
                    <section class="ranger-app-grid">
                        {move || {
                            let current_ledgers = ledgers.get();
                            let selected_ledger = selected_ledger_id.get();
                            view! {
                                <LedgersPage
                                    ledgers=current_ledgers
                                    selected_ledger_id=selected_ledger
                                    on_more=Callback::new(move |_| {
                                        open_device_settings();
                                        bill_editor.set(None);
                                    })
                                    on_select_ledger=Callback::new(open_ledger)
                                    on_new_ledger=Callback::new(move |_| overlay.set(Some(OverlayKind::CreateLedger)))
                                />
                            }
                        }}

                        {move || {
                            if let Some(detail) = ledger_detail.get() {
                                view! {
                                    <LedgerPage
                                        detail=detail
                                        on_back=Callback::new(move |_| {
                                            selected_ledger_id.set(None);
                                            ledger_detail.set(None);
                                        })
                                        on_more=Callback::new(move |_| open_ledger_settings())
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
                            }
                        }}

                        {move || {
                            if let Some(seed) = bill_editor.get() {
                                view! {
                                    <BillEditorPage
                                        title=if seed.prev_bill_id.is_some() {
                                            "Amend Bill".to_owned()
                                        } else {
                                            "New Bill".to_owned()
                                        }
                                        currency=seed.currency.clone()
                                        users=seed.users.clone()
                                        seed=seed
                                        on_back=Callback::new(move |_| bill_editor.set(None))
                                        on_save=Callback::new(save_bill)
                                    />
                                }
                                .into_any()
                            } else {
                                view! {
                                    <EmptyColumn
                                        title="Select a bill".to_owned()
                                        detail="Open a bill or ledger settings from the active ledger.".to_owned()
                                    />
                                }
                                .into_any()
                            }
                        }}
                    </section>

                    {move || render_settings_popup()}
                    {move || render_overlay()}
                </div>
            </main>
        }
        .into_any()
    };

    view! {
        {move || {
            if surface_mode.get() == SurfaceMode::Compact {
                view! {
                    <main class="app-shell">
                        {move || view! {
                            <StatusStrip
                                status=status_message.get()
                                error=error_message.get()
                                busy={ loading_count.get() != 0 }
                            />
                        }}
                        <div class="safe-content-area">
                            {move || render_compact_page()}
                            {move || render_settings_popup()}
                            {move || render_overlay()}
                        </div>
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
        .map(surface_mode_from_width)
        .unwrap_or(SurfaceMode::Ranger)
}

pub(crate) fn surface_mode_from_width(width: f64) -> SurfaceMode {
    if width < RANGER_BREAKPOINT {
        SurfaceMode::Compact
    } else {
        SurfaceMode::Ranger
    }
}

pub(crate) fn surface_mode_after_resize(_current: SurfaceMode, width: f64) -> SurfaceMode {
    surface_mode_from_width(width)
}

fn install_surface_mode_resize_listener(surface_mode: RwSignal<SurfaceMode>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let listener_window = window.clone();
    let resize_listener = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
        if let Ok(width) = listener_window.inner_width()
            && let Some(width) = width.as_f64()
        {
            surface_mode.set(surface_mode_after_resize(
                surface_mode.get_untracked(),
                width,
            ));
        }
    }));

    if window
        .add_event_listener_with_callback("resize", resize_listener.as_ref().unchecked_ref())
        .is_ok()
    {
        resize_listener.forget();
    }
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

fn new_bill_seed(currency: String, users: &[User]) -> BillEditorSeed {
    BillEditorSeed {
        currency,
        users: users.to_vec(),
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

fn amend_bill_seed(bill: &Bill, currency: String, users: &[User]) -> BillEditorSeed {
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
        currency,
        users: users.to_vec(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::LedgerSummary;

    fn ledger_summary(ledger_id: &str, name: &str) -> LedgerSummary {
        LedgerSummary {
            ledger_id: ledger_id.to_owned(),
            name: name.to_owned(),
            currency: "USD".to_owned(),
            created_at_ms: 0,
            updated_at_ms: 0,
            user_count: 0,
            latest_bill_at_ms: None,
        }
    }

    #[test]
    fn opening_device_settings_selects_device_tab_and_does_not_require_ledger() {
        let state = SettingsPopupState::open_device();

        assert_eq!(state.active_tab, SettingsTab::Device);
        assert_eq!(state.selected_ledger_id, None);
    }

    #[test]
    fn opening_ledger_settings_selects_ledger_tab_and_preselects_active_ledger() {
        let ledgers = vec![
            ledger_summary("ledger-a", "Groceries"),
            ledger_summary("ledger-b", "Handbag"),
        ];

        let state = SettingsPopupState::open_ledger(Some("ledger-b".to_owned()), &ledgers);

        assert_eq!(state.active_tab, SettingsTab::Ledger);
        assert_eq!(state.selected_ledger_id.as_deref(), Some("ledger-b"));
    }

    #[test]
    fn opening_ledger_settings_without_active_ledger_selects_first_ledger() {
        let ledgers = vec![
            ledger_summary("ledger-a", "Groceries"),
            ledger_summary("ledger-b", "Handbag"),
        ];

        let state = SettingsPopupState::open_ledger(None, &ledgers);

        assert_eq!(state.active_tab, SettingsTab::Ledger);
        assert_eq!(state.selected_ledger_id.as_deref(), Some("ledger-a"));
    }

    #[test]
    fn ranger_mode_keeps_columns_visible_while_settings_is_overlay() {
        let composition = ranger_composition(true);

        assert_eq!(
            composition.columns,
            RangerColumns {
                ledgers: true,
                bills: true,
                detail: true,
            }
        );
        assert!(composition.settings_overlay);
    }

    #[test]
    fn compact_mode_preserves_normal_priority_beneath_settings_overlay() {
        let composition = compact_composition(true, true, true);

        assert_eq!(composition.base_surface, CompactBaseSurface::Detail);
        assert!(composition.settings_overlay);
    }

    #[test]
    fn ledger_selector_changes_popup_state_without_changing_page_selection() {
        let ledgers = vec![
            ledger_summary("ledger-a", "Groceries"),
            ledger_summary("ledger-b", "Handbag"),
        ];
        let page_selection = Some("ledger-a".to_owned());
        let mut popup = SettingsPopupState::open_ledger(page_selection.clone(), &ledgers);

        popup.select_ledger("ledger-b".to_owned());

        assert_eq!(page_selection.as_deref(), Some("ledger-a"));
        assert_eq!(popup.selected_ledger_id.as_deref(), Some("ledger-b"));
    }

    #[test]
    fn width_below_ranger_breakpoint_uses_compact_mode() {
        assert_eq!(surface_mode_from_width(1199.0), SurfaceMode::Compact);
    }

    #[test]
    fn width_at_ranger_breakpoint_uses_ranger_mode() {
        assert_eq!(surface_mode_from_width(1200.0), SurfaceMode::Ranger);
    }

    #[test]
    fn very_wide_width_uses_ranger_mode() {
        assert_eq!(surface_mode_from_width(1800.0), SurfaceMode::Ranger);
    }

    #[test]
    fn resize_below_breakpoint_switches_from_ranger_to_compact() {
        assert_eq!(
            surface_mode_after_resize(SurfaceMode::Ranger, 900.0),
            SurfaceMode::Compact
        );
    }

    #[test]
    fn resize_at_breakpoint_switches_from_compact_to_ranger() {
        assert_eq!(
            surface_mode_after_resize(SurfaceMode::Compact, 1200.0),
            SurfaceMode::Ranger
        );
    }
}
