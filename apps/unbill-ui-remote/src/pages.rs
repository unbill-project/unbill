use crate::api::{self, LedgerDetail, LedgerSummary, SyncDevice, User};
use crate::app::SettingsTab;
use crate::components::{
    ActionButton, ButtonTone, CurrencyCombobox, FieldBlock, IconButton, IconButtonKind, ListRow,
    ModalSheet, ScreenFrame, SectionCard, SettingsNavGroup, SettingsNavItem,
};
use leptos::prelude::*;

#[component]
pub fn LedgersPage(
    ledgers: Vec<LedgerSummary>,
    selected_ledger_id: Option<String>,
    on_more: Callback<()>,
    on_select_ledger: Callback<String>,
    on_new_ledger: Callback<()>,
) -> impl IntoView {
    view! {
        <ScreenFrame
            header={view! {
                <div class="screen-leading"></div>
                <div class="screen-copy">
                    <h2 class="screen-title">"Ledgers"</h2>
                    <p class="screen-subtitle">"This server"</p>
                </div>
                <div class="screen-trailing">
                    <IconButton kind=IconButtonKind::More on_press=Callback::new(move |_| on_more.run(())) />
                </div>
            }.into_any()}
            footer={view! { <ActionButton label="New Ledger".to_owned() full_width=true on_press=Callback::new(move |_| on_new_ledger.run(())) /> }.into_any()}
        >
            <div class="stack-gap">
                {ledgers
                    .into_iter()
                    .map(|ledger| {
                        let ledger_id = ledger.ledger_id.clone();
                        let detail = ledger
                            .latest_bill_at_ms
                            .map(api::format_timestamp)
                            .unwrap_or_else(|| "No bills yet".to_owned());
                        view! {
                            <ListRow
                                title=ledger.name
                                meta=format!("{} users · {}", ledger.user_count, ledger.currency)
                                detail=detail
                                selected=selected_ledger_id
                                    .as_ref()
                                    .map(|selected| selected == &ledger_id)
                                    .unwrap_or(false)
                                on_press=Callback::new(move |_| on_select_ledger.run(ledger_id.clone()))
                            />
                        }
                    })
                    .collect_view()}
            </div>
        </ScreenFrame>
    }
}

#[component]
pub fn LedgerPage(
    detail: LedgerDetail,
    #[prop(optional)] show_back: bool,
    on_back: Callback<()>,
    on_more: Callback<()>,
    on_open_bill: Callback<String>,
    on_new_bill: Callback<()>,
) -> impl IntoView {
    let page_title = detail.summary.name.clone();
    let currency = detail.summary.currency.clone();
    let settlement_currency = currency.clone();

    view! {
        <ScreenFrame
            header={view! {
                <div class="screen-leading">
                    {show_back.then(|| view! { <IconButton kind=IconButtonKind::Back on_press=Callback::new(move |_| on_back.run(())) /> })}
                </div>
                <div class="screen-copy">
                    <h2 class="screen-title">{page_title}</h2>
                    <p class="screen-subtitle">{currency.clone()}</p>
                </div>
                <div class="screen-trailing">
                    <IconButton kind=IconButtonKind::More on_press=Callback::new(move |_| on_more.run(())) />
                </div>
            }.into_any()}
            footer={view! { <ActionButton label="New Bill".to_owned() full_width=true on_press=Callback::new(move |_| on_new_bill.run(())) /> }.into_any()}
        >
            <SectionCard
                title="Suggested transfers".to_owned()
            >
                {if detail.settlement.is_empty() {
                    view! { <div class="empty-copy">"All settled up."</div> }.into_any()
                } else {
                    view! {
                        <div class="stack-gap">
                            {detail
                                .settlement
                                .iter()
                                .map(|t| {
                                    view! {
                                        <ListRow
                                            title=t.from_name.clone()
                                            meta=format!("→ {}", t.to_name)
                                            detail=api::format_money(t.amount_cents, &settlement_currency)
                                        />
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any()
                }}
            </SectionCard>

            <SectionCard
                title="Bills".to_owned()
            >
                <div class="stack-gap">
                    {detail
                        .bills
                        .into_iter()
                        .map(|bill| {
                            let bill_id = bill.id.clone();
                            view! {
                                <ListRow
                                    title=if bill.description.is_empty() {
                                        "Untitled bill".to_owned()
                                    } else {
                                        bill.description
                                    }
                                    meta=format!("Paid by {}", bill.payers.first().map(|s| s.display_name.as_str()).unwrap_or("unknown"))
                                    detail=format!(
                                        "{} · {}",
                                        api::format_timestamp(bill.created_at_ms),
                                        api::format_money(bill.amount_cents, &currency)
                                    )
                                    on_press=Callback::new(move |_| on_open_bill.run(bill_id.clone()))
                                />
                            }
                        })
                        .collect_view()}
                </div>
            </SectionCard>
        </ScreenFrame>
    }
}

#[component]
fn SyncDeviceRow(
    title: String,
    node_id: String,
    #[prop(optional, into)] detail: Option<String>,
    on_sync: Callback<(String, Callback<()>)>,
) -> impl IntoView {
    let working = RwSignal::new(false);
    let done = Callback::new(move |_| working.set(false));
    view! {
        <div class="data-row split-row">
            <div class="row-copy">
                <p class="row-title">{title}</p>
                <p class="row-meta mono-copy">{node_id.clone()}</p>
                {detail.map(|d| view! { <p class="row-detail">{d}</p> })}
            </div>
            <IconButton
                kind=IconButtonKind::Sync
                tone=ButtonTone::Quiet
                working=working
                on_press=Callback::new(move |_| {
                    working.set(true);
                    on_sync.run((node_id.clone(), done));
                })
            />
        </div>
    }
}

#[component]
pub fn SettingsPopup(
    device_id: String,
    ledgers: Vec<LedgerSummary>,
    devices: Vec<SyncDevice>,
    active_tab: SettingsTab,
    mobile_in_content: bool,
    selected_ledger_id: Option<String>,
    ledger_detail: Option<LedgerDetail>,
    invitation_url: Option<String>,
    on_close: Callback<()>,
    on_select_tab: Callback<SettingsTab>,
    on_mobile_back: Callback<()>,
    on_select_ledger: Callback<String>,
    on_join_ledger: Callback<()>,
    on_add_ledger_user: Callback<()>,
    on_sync_device: Callback<(String, Callback<()>)>,
    on_create_invitation: Callback<()>,
    on_copy_invitation: Callback<()>,
) -> impl IntoView {
    let sidebar_class = if mobile_in_content {
        "settings-sidebar settings-sidebar-hidden"
    } else {
        "settings-sidebar"
    };
    let content_class = if mobile_in_content {
        "settings-content-area"
    } else {
        "settings-content-area settings-content-area-hidden"
    };
    let tab_label = match active_tab {
        SettingsTab::General => "General".to_owned(),
        SettingsTab::Ledger => ledgers
            .iter()
            .find(|l| selected_ledger_id.as_deref() == Some(l.ledger_id.as_str()))
            .map(|l| l.name.clone())
            .unwrap_or_else(|| "Ledger".to_owned()),
    };
    let selected_id_for_sidebar = selected_ledger_id.clone();

    view! {
        <div class="settings-overlay">
            <div class="settings-backdrop"></div>
            <section class="settings-panel">
                <header class="settings-header">
                    <div class="settings-title-block">
                        <h2 class="settings-title">"Settings"</h2>
                    </div>
                    <IconButton
                        kind=IconButtonKind::Close
                        on_press=Callback::new(move |_| on_close.run(()))
                    />
                </header>

                <div class="settings-layout">
                    <div class=sidebar_class>
                        <SettingsNavItem
                            label="General"
                            active=active_tab == SettingsTab::General
                            on_press=Callback::new(move |_| on_select_tab.run(SettingsTab::General))
                        />
                        <SettingsNavGroup title="Ledgers">
                            {ledgers
                                .into_iter()
                                .map(move |ledger| {
                                    let ledger_id = ledger.ledger_id.clone();
                                    let is_active = active_tab == SettingsTab::Ledger
                                        && selected_id_for_sidebar.as_deref() == Some(ledger.ledger_id.as_str());
                                    view! {
                                        <SettingsNavItem
                                            label=ledger.name
                                            active=is_active
                                            on_press=Callback::new(move |_| {
                                                on_select_tab.run(SettingsTab::Ledger);
                                                on_select_ledger.run(ledger_id.clone());
                                            })
                                        />
                                    }
                                })
                                .collect_view()}
                        </SettingsNavGroup>
                    </div>
                    <div class=content_class>
                        <div class="settings-mobile-back">
                            <IconButton
                                kind=IconButtonKind::Back
                                on_press=Callback::new(move |_| on_mobile_back.run(()))
                            />
                            <span class="settings-title">{tab_label}</span>
                        </div>
                        <div class="settings-body">
                    {if active_tab == SettingsTab::General {
                        view! {
                            <div class="settings-grid">
                                <SectionCard title="Device".to_owned()>
                                    <div class="data-row">
                                        <div class="row-copy">
                                            <p class="row-title">"Device ID"</p>
                                            <p class="row-meta mono-copy">{device_id}</p>
                                        </div>
                                    </div>
                                </SectionCard>

                                <SectionCard title="Ledger import".to_owned()>
                                    <ActionButton
                                        label="Paste Invitation".to_owned()
                                        tone=ButtonTone::Secondary
                                        on_press=Callback::new(move |_| on_join_ledger.run(()))
                                    />
                                </SectionCard>

                                <SectionCard title="Known devices".to_owned()>
                                    <div class="stack-gap">
                                        {if devices.is_empty() {
                                            view! { <div class="empty-copy">"No known devices."</div> }.into_any()
                                        } else {
                                            devices
                                                .into_iter()
                                                .map(|device| {
                                                    let node_id = device.node_id.clone();
                                                    let title = if device.label.trim().is_empty() {
                                                        "Unnamed device".to_owned()
                                                    } else {
                                                        device.label
                                                    };
                                                    let detail = if device.ledger_names.is_empty() {
                                                        "No shared ledgers".to_owned()
                                                    } else {
                                                        device.ledger_names.join(", ")
                                                    };
                                                    view! {
                                                        <SyncDeviceRow
                                                            title=title
                                                            node_id=node_id
                                                            detail=detail
                                                            on_sync=on_sync_device
                                                        />
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }}
                                    </div>
                                </SectionCard>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! {
                            <div class="settings-grid">
                                {if let Some(detail) = ledger_detail.clone() {
                                    view! {
                                        <SectionCard title="Users".to_owned()>
                                            <div class="stack-gap">
                                                {if detail.users.is_empty() {
                                                    view! { <div class="empty-copy">"No users."</div> }.into_any()
                                                } else {
                                                    detail
                                                        .users
                                                        .iter()
                                                        .map(|user| {
                                                            view! {
                                                                <div class="data-row">
                                                                    <div class="row-copy">
                                                                        <p class="row-title">{user.display_name.clone()}</p>
                                                                        <p class="row-meta mono-copy">{user.user_id.clone()}</p>
                                                                    </div>
                                                                </div>
                                                            }
                                                        })
                                                        .collect_view()
                                                        .into_any()
                                                }}

                                                <ActionButton
                                                    label="Add User".to_owned()
                                                    tone=ButtonTone::Secondary
                                                    on_press=Callback::new(move |_| on_add_ledger_user.run(()))
                                                />
                                            </div>
                                        </SectionCard>
                                    }
                                    .into_any()
                                } else {
                                    view! { <div /> }.into_any()
                                }}

                                {if let Some(detail) = ledger_detail {
                                    let authorized_devices = detail.devices.clone();
                                    view! {
                                        <SectionCard title="Authorized devices".to_owned()>
                                            <div class="stack-gap">
                                                {if authorized_devices.is_empty() {
                                                    view! { <div class="empty-copy">"No authorized devices."</div> }.into_any()
                                                } else {
                                                    authorized_devices
                                                        .into_iter()
                                                        .map(|device| {
                                                            let node_id = device.node_id.clone();
                                                            let title = if device.label.trim().is_empty() {
                                                                "Unnamed device".to_owned()
                                                            } else {
                                                                device.label
                                                            };
                                                            view! {
                                                                <SyncDeviceRow
                                                                    title=title
                                                                    node_id=node_id
                                                                    on_sync=on_sync_device
                                                                />
                                                            }
                                                        })
                                                        .collect_view()
                                                        .into_any()
                                                }}
                                            </div>
                                        </SectionCard>

                                        <SectionCard title="Device invitation".to_owned()>
                                            <div class="stack-gap">
                                                <ActionButton
                                                    label="Create Invitation".to_owned()
                                                    tone=ButtonTone::Secondary
                                                    on_press=Callback::new(move |_| on_create_invitation.run(()))
                                                />

                                                {invitation_url
                                                    .map(|url| {
                                                        view! {
                                                            <div class="result-panel">
                                                                <pre class="invite-url">{url.clone()}</pre>
                                                                <div class="result-actions">
                                                                    <IconButton
                                                                        kind=IconButtonKind::CopyUrl
                                                                        tone=ButtonTone::Quiet
                                                                        on_press=Callback::new(move |_| on_copy_invitation.run(()))
                                                                    />
                                                                </div>
                                                            </div>
                                                        }
                                                        .into_any()
                                                    })}
                                            </div>
                                        </SectionCard>
                                    }
                                    .into_any()
                                } else if selected_ledger_id.is_some() {
                                    view! { <div class="empty-copy">"Loading ledger."</div> }.into_any()
                                } else {
                                    view! { <div class="empty-copy">"Select a ledger."</div> }.into_any()
                                }}
                            </div>
                        }
                        .into_any()
                    }}
                        </div>
                    </div>
                </div>
            </section>
        </div>
    }
}

#[component]
pub fn CreateLedgerSheet(
    on_cancel: Callback<()>,
    on_submit: Callback<(String, String)>,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let currency = RwSignal::new("USD".to_owned());

    view! {
        <ModalSheet
            title="Create Ledger".to_owned()
            on_close=Callback::new(move |_| on_cancel.run(()))
        >
            <div class="stack-gap">
                <FieldBlock label="Ledger name".to_owned()>
                    <input
                        class="ui-input"
                        prop:value=move || name.get()
                        on:input=move |event| name.set(event_target_value(&event))
                    />
                </FieldBlock>
                <FieldBlock label="Currency".to_owned()>
                    <CurrencyCombobox value=currency />
                </FieldBlock>
                <ActionButton
                    label="Create Ledger".to_owned()
                    full_width=true
                    on_press=Callback::new(move |_| on_submit.run((name.get(), currency.get())))
                />
            </div>
        </ModalSheet>
    }
}

#[component]
pub fn AddLedgerUserSheet(
    all_users: Vec<User>,
    ledger_users: Vec<User>,
    on_cancel: Callback<()>,
    on_submit: Callback<String>,
    on_create_user: Callback<String>,
) -> impl IntoView {
    let available_users = all_users
        .into_iter()
        .filter(|user| {
            !ledger_users
                .iter()
                .any(|ledger_user| ledger_user.user_id == user.user_id)
        })
        .collect::<Vec<_>>();

    let new_name = RwSignal::new(String::new());

    view! {
        <ModalSheet
            title="Add User".to_owned()
            on_close=Callback::new(move |_| on_cancel.run(()))
        >
            <div class="stack-gap">
                <SectionCard title="Create new user".to_owned()>
                    <div class="stack-gap">
                        <FieldBlock label="Display name".to_owned()>
                            <input
                                class="ui-input"
                                prop:value=move || new_name.get()
                                on:input=move |event| new_name.set(event_target_value(&event))
                            />
                        </FieldBlock>
                        <ActionButton
                            label="Create".to_owned()
                            tone=ButtonTone::Secondary
                            on_press=Callback::new(move |_| {
                                let name = new_name.get();
                                if !name.trim().is_empty() {
                                    on_create_user.run(name);
                                }
                            })
                        />
                    </div>
                </SectionCard>

                {if available_users.is_empty() {
                    view! { <div class="empty-copy">"No users from other ledgers to add."</div> }.into_any()
                } else {
                    view! {
                        <SectionCard title="Add existing user".to_owned()>
                            {available_users
                                .into_iter()
                                .map(|user| {
                                    let user_id = user.user_id.clone();
                                    view! {
                                        <div class="data-row split-row">
                                            <div class="row-copy">
                                                <p class="row-title">{user.display_name}</p>
                                                <p class="row-meta mono-copy">{user_id.clone()}</p>
                                            </div>
                                            <IconButton
                                                kind=IconButtonKind::Add
                                                tone=ButtonTone::Secondary
                                                on_press=Callback::new(move |_| on_submit.run(user_id.clone()))
                                            />
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </SectionCard>
                    }
                    .into_any()
                }}
            </div>
        </ModalSheet>
    }
}

#[component]
pub fn JoinLedgerSheet(
    initial_url: String,
    on_cancel: Callback<()>,
    on_submit: Callback<(String, String)>,
) -> impl IntoView {
    let url = RwSignal::new(initial_url);
    let label = RwSignal::new(String::new());

    view! {
        <ModalSheet
            title="Join Ledger".to_owned()
            on_close=Callback::new(move |_| on_cancel.run(()))
        >
            <div class="stack-gap">
                <FieldBlock label="Invitation URL".to_owned()>
                    <textarea
                        class="ui-textarea"
                        prop:value=move || url.get()
                        on:input=move |event| url.set(event_target_value(&event))
                    />
                </FieldBlock>
                <FieldBlock label="Local device label".to_owned()>
                    <input
                        class="ui-input"
                        prop:value=move || label.get()
                        on:input=move |event| label.set(event_target_value(&event))
                    />
                </FieldBlock>
                <ActionButton
                    label="Join Ledger".to_owned()
                    full_width=true
                    on_press=Callback::new(move |_| on_submit.run((url.get(), label.get())))
                />
            </div>
        </ModalSheet>
    }
}
