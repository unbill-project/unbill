use crate::api::{self, LedgerDetail, LedgerSummary, SyncDevice, User};
use crate::app::{
    BillEditorSeed, BillSaveRequest, SettingsTab, ShareMode, derived_share_preview,
    parse_amount_text, share_lookup_shares,
};
use crate::components::{
    ActionButton, ButtonTone, CurrencyCombobox, FieldBlock, IconButton, IconButtonKind, ListRow,
    ModalSheet, ScreenFrame, SectionCard, TagPill,
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
            title="Ledgers".to_owned()
            subtitle="This device".to_owned()
            trailing={view! { <IconButton kind=IconButtonKind::More on_press=Callback::new(move |_| on_more.run(())) /> }.into_any()}
            footer={view! { <ActionButton label="New Ledger".to_owned() full_width=true on_press=Callback::new(move |_| on_new_ledger.run(())) /> }.into_any()}
        >
            <SectionCard
                title="Ledgers".to_owned()
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
            </SectionCard>
        </ScreenFrame>
    }
}

#[component]
pub fn LedgerPage(
    detail: LedgerDetail,
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
            title=page_title
            subtitle=currency.clone()
            leading={view! { <IconButton kind=IconButtonKind::Back on_press=Callback::new(move |_| on_back.run(())) /> }.into_any()}
            trailing={view! { <IconButton kind=IconButtonKind::More on_press=Callback::new(move |_| on_more.run(())) /> }.into_any()}
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
pub fn SettingsPopup(
    device_id: String,
    ledgers: Vec<LedgerSummary>,
    devices: Vec<SyncDevice>,
    active_tab: SettingsTab,
    selected_ledger_id: Option<String>,
    ledger_detail: Option<LedgerDetail>,
    invitation_url: Option<String>,
    on_close: Callback<()>,
    on_select_tab: Callback<SettingsTab>,
    on_select_ledger: Callback<String>,
    on_join_ledger: Callback<()>,
    on_add_ledger_user: Callback<()>,
    on_sync_device: Callback<String>,
    on_create_invitation: Callback<()>,
    on_copy_invitation: Callback<()>,
) -> impl IntoView {
    let device_tab_class = if active_tab == SettingsTab::Device {
        "tab-button tab-button-active"
    } else {
        "tab-button"
    };
    let ledger_tab_class = if active_tab == SettingsTab::Ledger {
        "tab-button tab-button-active"
    } else {
        "tab-button"
    };
    let selected_for_select = selected_ledger_id.clone().unwrap_or_default();

    view! {
        <div class="settings-overlay">
            <div class="settings-backdrop"></div>
            <section class="settings-panel">
                <header class="settings-header">
                    <div class="settings-title-block">
                        <h2 class="settings-title">"Settings"</h2>
                        <p class="settings-subtitle">{device_id.clone()}</p>
                    </div>
                    <IconButton
                        kind=IconButtonKind::Close
                        on_press=Callback::new(move |_| on_close.run(()))
                    />
                </header>

                <div class="settings-tabs">
                    <button
                        type="button"
                        class=device_tab_class
                        on:click=move |_| on_select_tab.run(SettingsTab::Device)
                    >
                        "Device Settings"
                    </button>
                    <button
                        type="button"
                        class=ledger_tab_class
                        on:click=move |_| on_select_tab.run(SettingsTab::Ledger)
                    >
                        "Ledger Settings"
                    </button>
                </div>

                <div class="settings-body">
                    {if active_tab == SettingsTab::Device {
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
                                                        <div class="data-row split-row">
                                                            <div class="row-copy">
                                                                <p class="row-title">{title}</p>
                                                                <p class="row-meta mono-copy">{node_id.clone()}</p>
                                                                <p class="row-detail">{detail}</p>
                                                            </div>
                                                            <IconButton
                                                                kind=IconButtonKind::Sync
                                                                tone=ButtonTone::Quiet
                                                                on_press=Callback::new(move |_| on_sync_device.run(node_id.clone()))
                                                            />
                                                        </div>
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }}
                                    </div>
                                </SectionCard>

                                <SectionCard title="Ledger import".to_owned()>
                                    <ActionButton
                                        label="Join Ledger".to_owned()
                                        tone=ButtonTone::Secondary
                                        on_press=Callback::new(move |_| on_join_ledger.run(()))
                                    />
                                </SectionCard>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="settings-grid">
                                <SectionCard title="Ledger".to_owned()>
                                    {if ledgers.is_empty() {
                                        view! { <div class="empty-copy">"No ledgers available."</div> }.into_any()
                                    } else {
                                        view! {
                                            <FieldBlock label="Selected ledger".to_owned()>
                                                <select
                                                    class="ui-select"
                                                    prop:value=move || selected_for_select.clone()
                                                    on:change=move |event| on_select_ledger.run(event_target_value(&event))
                                                >
                                                    {ledgers
                                                        .clone()
                                                        .into_iter()
                                                        .map(|ledger| {
                                                            view! {
                                                                <option value=ledger.ledger_id>{ledger.name}</option>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </select>
                                            </FieldBlock>
                                        }
                                            .into_any()
                                    }}
                                </SectionCard>

                                {if let Some(detail) = ledger_detail {
                                    let sync_devices = detail.devices.clone();
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

                                        <SectionCard title="Authorized devices".to_owned()>
                                            <div class="stack-gap">
                                                {if sync_devices.is_empty() {
                                                    view! { <div class="empty-copy">"No authorized devices."</div> }.into_any()
                                                } else {
                                                    sync_devices
                                                        .into_iter()
                                                        .map(|device| {
                                                            let node_id = device.node_id.clone();
                                                            let title = if device.label.trim().is_empty() {
                                                                "Unnamed device".to_owned()
                                                            } else {
                                                                device.label
                                                            };
                                                            view! {
                                                                <div class="data-row split-row">
                                                                    <div class="row-copy">
                                                                        <p class="row-title">{title}</p>
                                                                        <p class="row-meta mono-copy">{node_id.clone()}</p>
                                                                    </div>
                                                                    <IconButton
                                                                        kind=IconButtonKind::Sync
                                                                        tone=ButtonTone::Quiet
                                                                        on_press=Callback::new(move |_| on_sync_device.run(node_id.clone()))
                                                                    />
                                                                </div>
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
            </section>
        </div>
    }
}

#[component]
pub fn BillEditorPage(
    title: String,
    currency: String,
    users: Vec<User>,
    seed: BillEditorSeed,
    on_back: Callback<()>,
    on_save: Callback<BillSaveRequest>,
) -> impl IntoView {
    let _ = users;
    let description = RwSignal::new(seed.description);
    let amount_text = RwSignal::new(seed.amount_text);
    let payer_mode = RwSignal::new(seed.payer_mode);
    let payer_rows = RwSignal::new(seed.payer_rows);
    let share_mode = RwSignal::new(seed.share_mode);
    let share_rows = RwSignal::new(seed.share_rows);
    let validation_error = RwSignal::new(None::<String>);
    let currency_field_value = currency.clone();
    let payer_currency = currency.clone();
    let split_currency = currency.clone();

    let save_click = move |_| {
        let amount_cents = match parse_amount_text(&amount_text.get()) {
            Ok(value) => value,
            Err(error) => {
                validation_error.set(Some(error));
                return;
            }
        };

        let active_payer_rows = payer_rows
            .get()
            .into_iter()
            .filter(|row| row.included)
            .collect::<Vec<_>>();

        if active_payer_rows.is_empty() {
            validation_error.set(Some("Select at least one payer before saving.".to_owned()));
            return;
        }

        let payers = active_payer_rows
            .into_iter()
            .map(|row| crate::api::BillShareInput {
                user_id: row.user_id,
                shares: if payer_mode.get() == ShareMode::Equal {
                    1
                } else {
                    row.shares
                },
            })
            .collect::<Vec<_>>();

        if payers.iter().any(|item| item.shares == 0) {
            validation_error.set(Some(
                "Custom payer shares must be greater than zero.".to_owned(),
            ));
            return;
        }

        let active_share_rows = share_rows
            .get()
            .into_iter()
            .filter(|share_row| share_row.included)
            .collect::<Vec<_>>();

        if active_share_rows.is_empty() {
            validation_error.set(Some("Select at least one payee before saving.".to_owned()));
            return;
        }

        let shares = active_share_rows
            .into_iter()
            .map(|share_row| crate::api::BillShareInput {
                user_id: share_row.user_id,
                shares: if share_mode.get() == ShareMode::Equal {
                    1
                } else {
                    share_row.shares
                },
            })
            .collect::<Vec<_>>();

        if shares.iter().any(|item| item.shares == 0) {
            validation_error.set(Some("Custom shares must be greater than zero.".to_owned()));
            return;
        }

        validation_error.set(None);
        on_save.run(BillSaveRequest {
            prev_bill_id: seed.prev_bill_id.clone(),
            description: description.get(),
            payers,
            amount_cents,
            shares,
        });
    };

    view! {
        <ScreenFrame
            title=title
            subtitle="Bill details".to_owned()
            leading={view! { <IconButton kind=IconButtonKind::Back on_press=Callback::new(move |_| on_back.run(())) /> }.into_any()}
            trailing={view! { <IconButton kind=IconButtonKind::Save tone=ButtonTone::Secondary on_press=Callback::new(save_click) /> }.into_any()}
        >
            <div class="stack-gap">
                <SectionCard
                    title="Core fields".to_owned()
                >
                    <div class="stack-gap">
                        <FieldBlock label="Description".to_owned()>
                            <input
                                class="ui-input"
                                prop:value=move || description.get()
                                on:input=move |event| description.set(event_target_value(&event))
                            />
                        </FieldBlock>

                        <div class="field-grid">
                            <FieldBlock label="Amount".to_owned()>
                                <input
                                    class="ui-input"
                                    prop:value=move || amount_text.get()
                                    on:input=move |event| amount_text.set(event_target_value(&event))
                                />
                            </FieldBlock>
                            <FieldBlock label="Currency".to_owned()>
                                <input class="ui-input" value=currency_field_value.clone() readonly />
                            </FieldBlock>
                        </div>
                    </div>
                </SectionCard>

                <SectionCard
                    title="Who paid".to_owned()
                >
                    <div class="stack-gap">
                        <div class="chip-row">
                            <button
                                type="button"
                                class=move || {
                                    if payer_mode.get() == ShareMode::Equal {
                                        "tag-pill tag-pill-active"
                                    } else {
                                        "tag-pill"
                                    }
                                }
                                on:click=move |_| payer_mode.set(ShareMode::Equal)
                            >
                                "Equal split"
                            </button>
                            <button
                                type="button"
                                class=move || {
                                    if payer_mode.get() == ShareMode::Custom {
                                        "tag-pill tag-pill-active"
                                    } else {
                                        "tag-pill"
                                    }
                                }
                                on:click=move |_| payer_mode.set(ShareMode::Custom)
                            >
                                "Custom shares"
                            </button>
                        </div>

                        {move || {
                            let current_mode = payer_mode.get();
                            let current_amount = parse_amount_text(&amount_text.get()).unwrap_or(0);
                            let current_rows = payer_rows.get();
                            let preview = derived_share_preview(current_amount, current_mode, &current_rows);

                            current_rows
                                .into_iter()
                                .map(|row| {
                                    let user_id = row.user_id.clone();
                                    let toggle_user_id = user_id.clone();
                                    let share_user_id = user_id.clone();
                                    let share_value_user_id = user_id.clone();
                                    let display_name = row.display_name.clone();
                                    let preview_text = preview
                                        .iter()
                                        .find(|(pid, _)| pid == &user_id)
                                        .map(|(_, cents)| api::format_money(*cents, &payer_currency))
                                        .unwrap_or_else(|| format!("{} 0.00", payer_currency));

                                    view! {
                                        <div class="share-row">
                                            <label class="share-toggle">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=row.included
                                                    on:change=move |event| {
                                                        let checked = event_target_checked(&event);
                                                        payer_rows.update(|items| {
                                                            if let Some(item) = items.iter_mut().find(|item| item.user_id == toggle_user_id) {
                                                                item.included = checked;
                                                            }
                                                        });
                                                    }
                                                />
                                                <span>{display_name}</span>
                                            </label>

                                            <div class="share-side">
                                                {if current_mode == ShareMode::Custom {
                                                    view! {
                                                        <input
                                                            class="share-input"
                                                            prop:value=share_lookup_shares(&payer_rows.get(), &share_value_user_id).to_string()
                                                            on:input=move |event| {
                                                                let value = event_target_value(&event)
                                                                    .parse::<u32>()
                                                                    .ok()
                                                                    .filter(|v| *v > 0)
                                                                    .unwrap_or(1);
                                                                payer_rows.update(|items| {
                                                                    if let Some(item) = items.iter_mut().find(|item| item.user_id == share_user_id) {
                                                                        item.shares = value;
                                                                    }
                                                                });
                                                            }
                                                        />
                                                    }
                                                        .into_any()
                                                } else {
                                                    view! { <TagPill label="1 share".to_owned() active=true /> }.into_any()
                                                }}
                                                <span class="share-amount">{preview_text}</span>
                                            </div>
                                        </div>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </SectionCard>

                <SectionCard
                    title="Share split".to_owned()
                >
                    <div class="stack-gap">
                        <div class="chip-row">
                            <button
                                type="button"
                                class=move || {
                                    if share_mode.get() == ShareMode::Equal {
                                        "tag-pill tag-pill-active"
                                    } else {
                                        "tag-pill"
                                    }
                                }
                                on:click=move |_| share_mode.set(ShareMode::Equal)
                            >
                                "Equal split"
                            </button>
                            <button
                                type="button"
                                class=move || {
                                    if share_mode.get() == ShareMode::Custom {
                                        "tag-pill tag-pill-active"
                                    } else {
                                        "tag-pill"
                                    }
                                }
                                on:click=move |_| share_mode.set(ShareMode::Custom)
                            >
                                "Custom shares"
                            </button>
                        </div>

                        {move || {
                            let current_mode = share_mode.get();
                            let current_amount = parse_amount_text(&amount_text.get()).unwrap_or(0);
                            let current_rows = share_rows.get();
                            let preview = derived_share_preview(current_amount, current_mode, &current_rows);

                            current_rows
                                .into_iter()
                                .map(|share_row| {
                                    let user_id = share_row.user_id.clone();
                                    let toggle_user_id = user_id.clone();
                                    let share_user_id = user_id.clone();
                                    let share_value_user_id = user_id.clone();
                                    let display_name = share_row.display_name.clone();
                                    let preview_text = preview
                                        .iter()
                                        .find(|(preview_user_id, _)| preview_user_id == &user_id)
                                        .map(|(_, cents)| api::format_money(*cents, &split_currency))
                                        .unwrap_or_else(|| format!("{} 0.00", split_currency));

                                    view! {
                                        <div class="share-row">
                                            <label class="share-toggle">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=share_row.included
                                                    on:change=move |event| {
                                                        let checked = event_target_checked(&event);
                                                        share_rows.update(|items| {
                                                            if let Some(item) = items.iter_mut().find(|item| item.user_id == toggle_user_id) {
                                                                item.included = checked;
                                                            }
                                                        });
                                                    }
                                                />
                                                <span>{display_name}</span>
                                            </label>

                                            <div class="share-side">
                                                {if current_mode == ShareMode::Custom {
                                                    view! {
                                                        <input
                                                            class="share-input"
                                                            prop:value=share_lookup_shares(&share_rows.get(), &share_value_user_id).to_string()
                                                            on:input=move |event| {
                                                                let value = event_target_value(&event)
                                                                    .parse::<u32>()
                                                                    .ok()
                                                                    .filter(|value| *value > 0)
                                                                    .unwrap_or(1);
                                                                share_rows.update(|items| {
                                                                    if let Some(item) = items.iter_mut().find(|item| item.user_id == share_user_id) {
                                                                        item.shares = value;
                                                                    }
                                                                });
                                                            }
                                                        />
                                                    }
                                                        .into_any()
                                                } else {
                                                    view! { <TagPill label="1 share".to_owned() active=true /> }.into_any()
                                                }}
                                                <span class="share-amount">{preview_text}</span>
                                            </div>
                                        </div>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </SectionCard>

                {move || {
                    validation_error
                        .get()
                        .map(|error| view! { <p class="form-error">{error}</p> }.into_any())
                }}
            </div>
        </ScreenFrame>
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
