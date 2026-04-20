use crate::api::{
    self, AddIdentityInput, AddMemberInput, AppBootstrap, Bill, BillShareInput, JoinLedgerInput,
    LedgerDetail, LedgerSummary, Member, SaveBillInput,
};
use crate::pages::{
    AddIdentitySheet, AddMemberSheet, BillEditorPage, CreateLedgerSheet, DeviceSettingsPage,
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
    AddIdentity,
    JoinLedger { url: String },
    AddMember,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShareMode {
    Equal,
    Custom,
}

#[derive(Clone, PartialEq)]
pub(crate) struct ParticipantDraft {
    pub(crate) user_id: String,
    pub(crate) display_name: String,
    pub(crate) included: bool,
    pub(crate) shares: u32,
}

#[derive(Clone, PartialEq)]
pub(crate) struct BillEditorSeed {
    pub(crate) prev_bill_id: Option<String>,
    pub(crate) description: String,
    pub(crate) payer_user_id: Option<String>,
    pub(crate) amount_text: String,
    pub(crate) share_mode: ShareMode,
    pub(crate) participants: Vec<ParticipantDraft>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct BillSaveRequest {
    pub(crate) prev_bill_id: Option<String>,
    pub(crate) description: String,
    pub(crate) payer_user_id: String,
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

    let load_selected_ledger = {
        let ledger_detail = ledger_detail;
        let selected_ledger_id = selected_ledger_id;
        let error_message = error_message;
        let busy = busy;
        move |ledger_id: String| {
            selected_ledger_id.set(Some(ledger_id.clone()));
            busy.set(true);
            spawn_local({
                let ledger_detail = ledger_detail;
                let error_message = error_message;
                let busy = busy;
                async move {
                    match api::load_ledger_detail(&ledger_id).await {
                        Ok(detail) => {
                            ledger_detail.set(Some(detail));
                            error_message.set(None);
                        }
                        Err(error) => error_message.set(Some(error)),
                    }
                    busy.set(false);
                }
            });
        }
    };

    let reload_bootstrap = {
        let bootstrap = bootstrap;
        let selected_ledger_id = selected_ledger_id;
        let ledger_detail = ledger_detail;
        let error_message = error_message;
        let busy = busy;
        move || {
            busy.set(true);
            spawn_local({
                let bootstrap = bootstrap;
                let selected_ledger_id = selected_ledger_id;
                let ledger_detail = ledger_detail;
                let error_message = error_message;
                let busy = busy;
                async move {
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

                            if let Some(ledger_id) = selected {
                                if selection_exists {
                                    match api::load_ledger_detail(&ledger_id).await {
                                        Ok(detail) => ledger_detail.set(Some(detail)),
                                        Err(error) => {
                                            ledger_detail.set(None);
                                            error_message.set(Some(error));
                                        }
                                    }
                                }
                            }

                            bootstrap.set(Some(data));
                            error_message.set(None);
                        }
                        Err(error) => error_message.set(Some(error)),
                    }
                    busy.set(false);
                }
            });
        }
    };

    reload_bootstrap();

    let open_ledger = {
        let device_settings_open = device_settings_open;
        let ledger_settings_open = ledger_settings_open;
        let invitation_url = invitation_url;
        let bill_editor = bill_editor;
        let load_selected_ledger = load_selected_ledger.clone();
        move |ledger_id: String| {
            device_settings_open.set(false);
            ledger_settings_open.set(false);
            invitation_url.set(None);
            bill_editor.set(None);
            load_selected_ledger(ledger_id);
        }
    };

    let open_new_bill = {
        let bill_editor = bill_editor;
        let error_message = error_message;
        let ledger_detail = ledger_detail;
        move || {
            if let Some(detail) = ledger_detail.get() {
                if detail.members.is_empty() {
                    error_message.set(Some(
                        "Add at least one ledger member before creating a bill.".to_owned(),
                    ));
                    return;
                }
                bill_editor.set(Some(new_bill_seed(&detail.members)));
                error_message.set(None);
            }
        }
    };

    let open_bill_amend = {
        let ledger_detail = ledger_detail;
        let bill_editor = bill_editor;
        move |bill_id: String| {
            if let Some(detail) = ledger_detail.get() {
                if let Some(bill) = detail.bills.iter().find(|item| item.id == bill_id) {
                    bill_editor.set(Some(amend_bill_seed(bill, &detail.members)));
                }
            }
        }
    };

    let save_bill = {
        let selected_ledger_id = selected_ledger_id;
        let bill_editor = bill_editor;
        let error_message = error_message;
        let status_message = status_message;
        let busy = busy;
        let reload_bootstrap = reload_bootstrap.clone();
        let load_selected_ledger = load_selected_ledger.clone();
        move |request: BillSaveRequest| {
            if let Some(ledger_id) = selected_ledger_id.get() {
                busy.set(true);
                spawn_local({
                    let bill_editor = bill_editor;
                    let error_message = error_message;
                    let status_message = status_message;
                    let busy = busy;
                    let reload_bootstrap = reload_bootstrap.clone();
                    let load_selected_ledger = load_selected_ledger.clone();
                    async move {
                        match api::save_bill(SaveBillInput {
                            ledger_id: ledger_id.clone(),
                            description: request.description,
                            payer_user_id: request.payer_user_id,
                            amount_cents: request.amount_cents,
                            shares: request.shares,
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
                    }
                });
            }
        }
    };

    let create_invitation = {
        let selected_ledger_id = selected_ledger_id;
        let invitation_url = invitation_url;
        let error_message = error_message;
        let status_message = status_message;
        let busy = busy;
        move || {
            if let Some(ledger_id) = selected_ledger_id.get() {
                busy.set(true);
                spawn_local({
                    let invitation_url = invitation_url;
                    let error_message = error_message;
                    let status_message = status_message;
                    let busy = busy;
                    async move {
                        match api::create_invitation(&ledger_id).await {
                            Ok(url) => {
                                invitation_url.set(Some(url));
                                status_message.set(Some("Invitation URL generated.".to_owned()));
                                error_message.set(None);
                            }
                            Err(error) => error_message.set(Some(error)),
                        }
                        busy.set(false);
                    }
                });
            }
        }
    };

    let copy_invitation_url = {
        let invitation_url = invitation_url;
        let error_message = error_message;
        let status_message = status_message;
        move || {
            if let Some(url) = invitation_url.get() {
                spawn_local({
                    let error_message = error_message;
                    let status_message = status_message;
                    async move {
                        match api::write_clipboard_text(&url).await {
                            Ok(()) => {
                                status_message.set(Some("Invitation URL copied.".to_owned()));
                                error_message.set(None);
                            }
                            Err(error) => error_message.set(Some(error)),
                        }
                    }
                });
            }
        }
    };

    let open_join_from_clipboard = {
        let overlay = overlay;
        let error_message = error_message;
        move || {
            spawn_local({
                let overlay = overlay;
                let error_message = error_message;
                async move {
                    match api::read_clipboard_text().await {
                        Ok(url) if !url.trim().is_empty() => {
                            overlay.set(Some(OverlayKind::JoinLedger { url }));
                            error_message.set(None);
                        }
                        Ok(_) => error_message.set(Some("Clipboard is empty.".to_owned())),
                        Err(error) => error_message.set(Some(error)),
                    }
                }
            });
        }
    };

    let sync_device = {
        let error_message = error_message;
        let status_message = status_message;
        let busy = busy;
        let reload_bootstrap = reload_bootstrap.clone();
        let selected_ledger_id = selected_ledger_id;
        let load_selected_ledger = load_selected_ledger.clone();
        move |peer_node_id: String| {
            busy.set(true);
            spawn_local({
                let error_message = error_message;
                let status_message = status_message;
                let busy = busy;
                let reload_bootstrap = reload_bootstrap.clone();
                let selected_ledger_id = selected_ledger_id;
                let load_selected_ledger = load_selected_ledger.clone();
                async move {
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
                }
            });
        }
    };

    let render_overlay = move || {
        overlay.get().map(|sheet| match sheet {
            OverlayKind::CreateLedger => {
                view! {
                    <CreateLedgerSheet
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new({
                            let overlay = overlay;
                            let open_ledger = open_ledger.clone();
                            let reload_bootstrap = reload_bootstrap.clone();
                            let error_message = error_message;
                            let status_message = status_message;
                            let busy = busy;
                            move |(name, currency): (String, String)| {
                                busy.set(true);
                                spawn_local({
                                    let overlay = overlay;
                                    let open_ledger = open_ledger.clone();
                                    let reload_bootstrap = reload_bootstrap.clone();
                                    let error_message = error_message;
                                    let status_message = status_message;
                                    let busy = busy;
                                    async move {
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
                                    }
                                });
                            }
                        })
                    />
                }
                    .into_any()
            }
            OverlayKind::AddIdentity => {
                view! {
                    <AddIdentitySheet
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new({
                            let overlay = overlay;
                            let reload_bootstrap = reload_bootstrap.clone();
                            let error_message = error_message;
                            let status_message = status_message;
                            let busy = busy;
                            move |display_name: String| {
                                busy.set(true);
                                spawn_local({
                                    let overlay = overlay;
                                    let reload_bootstrap = reload_bootstrap.clone();
                                    let error_message = error_message;
                                    let status_message = status_message;
                                    let busy = busy;
                                    async move {
                                        match api::add_identity(AddIdentityInput { display_name }).await {
                                            Ok(_) => {
                                                overlay.set(None);
                                                reload_bootstrap();
                                                status_message.set(Some("Identity saved on this device.".to_owned()));
                                                error_message.set(None);
                                            }
                                            Err(error) => error_message.set(Some(error)),
                                        }
                                        busy.set(false);
                                    }
                                });
                            }
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
                        on_submit=Callback::new({
                            let overlay = overlay;
                            let reload_bootstrap = reload_bootstrap.clone();
                            let error_message = error_message;
                            let status_message = status_message;
                            let busy = busy;
                            move |(url, label): (String, String)| {
                                busy.set(true);
                                spawn_local({
                                    let overlay = overlay;
                                    let reload_bootstrap = reload_bootstrap.clone();
                                    let error_message = error_message;
                                    let status_message = status_message;
                                    let busy = busy;
                                    async move {
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
                                    }
                                });
                            }
                        })
                    />
                }
                    .into_any()
            }
            OverlayKind::AddMember => {
                view! {
                    <AddMemberSheet
                        on_cancel=Callback::new(move |_| overlay.set(None))
                        on_submit=Callback::new({
                            let overlay = overlay;
                            let selected_ledger_id = selected_ledger_id;
                            let error_message = error_message;
                            let status_message = status_message;
                            let busy = busy;
                            let load_selected_ledger = load_selected_ledger.clone();
                            let reload_bootstrap = reload_bootstrap.clone();
                            move |display_name: String| {
                                if let Some(ledger_id) = selected_ledger_id.get() {
                                    busy.set(true);
                                    spawn_local({
                                        let overlay = overlay;
                                        let error_message = error_message;
                                        let status_message = status_message;
                                        let busy = busy;
                                        let load_selected_ledger = load_selected_ledger.clone();
                                        let reload_bootstrap = reload_bootstrap.clone();
                                        async move {
                                            match api::add_member(AddMemberInput { ledger_id: ledger_id.clone(), display_name }).await {
                                                Ok(_) => {
                                                    overlay.set(None);
                                                    load_selected_ledger(ledger_id);
                                                    reload_bootstrap();
                                                    status_message.set(Some("Member added to ledger.".to_owned()));
                                                    error_message.set(None);
                                                }
                                                Err(error) => error_message.set(Some(error)),
                                            }
                                            busy.set(false);
                                        }
                                    });
                                }
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
                        members=ledger_detail
                            .get()
                            .map(|detail| detail.members)
                            .unwrap_or_default()
                        seed=seed
                        on_back=Callback::new(move |_| bill_editor.set(None))
                        on_save=Callback::new(move |request| save_bill(request))
                    />
                </div>
            }
            .into_any();
        }

        if ledger_settings_open.get() {
            if let Some(detail) = ledger_detail.get() {
                return view! {
                    <div class="app-shell">
                        <LedgerSettingsPage
                            detail=detail
                            invitation_url=invitation_url.get()
                            on_back=Callback::new(move |_| ledger_settings_open.set(false))
                            on_add_member=Callback::new(move |_| overlay.set(Some(OverlayKind::AddMember)))
                            on_create_invitation=Callback::new(move |_| create_invitation())
                            on_copy_invitation=Callback::new(move |_| copy_invitation_url())
                        />
                    </div>
                }
                .into_any();
            }
        }

        if device_settings_open.get() {
            return view! {
                <div class="app-shell">
                    <DeviceSettingsPage
                        identities=bootstrap.get().map(|data| data.identities).unwrap_or_default()
                        devices=bootstrap.get().map(|data| data.devices).unwrap_or_default()
                        on_back=Callback::new(move |_| device_settings_open.set(false))
                        on_add_identity=Callback::new(move |_| overlay.set(Some(OverlayKind::AddIdentity)))
                        on_import_ledger=Callback::new(move |_| open_join_from_clipboard())
                        on_scan_qr=Callback::new(move |_| open_join_from_clipboard())
                        on_sync_device=Callback::new(move |node_id| sync_device(node_id))
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
                        on_open_bill=Callback::new(move |bill_id| open_bill_amend(bill_id))
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
                    on_select_ledger=Callback::new(move |ledger_id| open_ledger(ledger_id))
                    on_new_ledger=Callback::new(move |_| overlay.set(Some(OverlayKind::CreateLedger)))
                />
            </div>
        }
        .into_any()
    };

    let render_ranger = move || {
        let ledgers = bootstrap.get().map(|data| data.ledgers).unwrap_or_default();
        let identities = bootstrap
            .get()
            .map(|data| data.identities)
            .unwrap_or_default();
        let selected_ledger = selected_ledger_id.get();

        let column_two = if device_settings_open.get() {
            view! {
                <DeviceSettingsPage
                    identities=identities
                    devices=bootstrap.get().map(|data| data.devices).unwrap_or_default()
                    on_back=Callback::new(move |_| device_settings_open.set(false))
                    on_add_identity=Callback::new(move |_| overlay.set(Some(OverlayKind::AddIdentity)))
                    on_import_ledger=Callback::new(move |_| open_join_from_clipboard())
                    on_scan_qr=Callback::new(move |_| open_join_from_clipboard())
                    on_sync_device=Callback::new(move |node_id| sync_device(node_id))
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
                    on_open_bill=Callback::new(move |bill_id| open_bill_amend(bill_id))
                    on_new_bill=Callback::new(move |_| open_new_bill())
                />
            }
            .into_any()
        } else {
            view! {
                <EmptyColumn
                    title="Select a ledger".to_owned()
                    detail="Choose a ledger to load bills and member state.".to_owned()
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
                    members=ledger_detail
                        .get()
                        .map(|detail| detail.members)
                        .unwrap_or_default()
                    seed=seed
                    on_back=Callback::new(move |_| bill_editor.set(None))
                    on_save=Callback::new(move |request| save_bill(request))
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
                        on_add_member=Callback::new(move |_| overlay.set(Some(OverlayKind::AddMember)))
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
                        on_select_ledger=Callback::new(move |ledger_id| open_ledger(ledger_id))
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

fn new_bill_seed(members: &[Member]) -> BillEditorSeed {
    BillEditorSeed {
        prev_bill_id: None,
        description: String::new(),
        payer_user_id: members.first().map(|member| member.user_id.clone()),
        amount_text: String::new(),
        share_mode: ShareMode::Equal,
        participants: members
            .iter()
            .map(|member| ParticipantDraft {
                user_id: member.user_id.clone(),
                display_name: member.display_name.clone(),
                included: true,
                shares: 1,
            })
            .collect(),
    }
}

fn amend_bill_seed(bill: &Bill, members: &[Member]) -> BillEditorSeed {
    let shares_by_user = bill
        .shares
        .iter()
        .map(|share| (share.user_id.clone(), share.shares))
        .collect::<std::collections::HashMap<_, _>>();
    let share_mode = if shares_by_user.values().all(|shares| *shares == 1) {
        ShareMode::Equal
    } else {
        ShareMode::Custom
    };

    BillEditorSeed {
        prev_bill_id: Some(bill.id.clone()),
        description: bill.description.clone(),
        payer_user_id: Some(bill.payer_user_id.clone()),
        amount_text: format!(
            "{}.{:02}",
            bill.amount_cents / 100,
            bill.amount_cents.abs() % 100
        ),
        share_mode,
        participants: members
            .iter()
            .map(|member| ParticipantDraft {
                user_id: member.user_id.clone(),
                display_name: member.display_name.clone(),
                included: shares_by_user.contains_key(&member.user_id),
                shares: shares_by_user.get(&member.user_id).copied().unwrap_or(1),
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

pub(crate) fn participant_lookup_shares(participants: &[ParticipantDraft], user_id: &str) -> u32 {
    participants
        .iter()
        .find(|participant| participant.user_id == user_id)
        .map(|participant| participant.shares)
        .unwrap_or(1)
}

pub(crate) fn derived_share_preview(
    amount_cents: i64,
    share_mode: ShareMode,
    participants: &[ParticipantDraft],
) -> Vec<(String, i64)> {
    let active = participants
        .iter()
        .filter(|participant| participant.included)
        .map(|participant| {
            (
                participant.user_id.clone(),
                if share_mode == ShareMode::Equal {
                    1
                } else {
                    participant.shares
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
