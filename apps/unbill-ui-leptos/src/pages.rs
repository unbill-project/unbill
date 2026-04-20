use crate::api::{self, Identity, LedgerDetail, LedgerSummary, Member, SyncDevice};
use crate::app::{
    BillEditorSeed, BillSaveRequest, ShareMode, derived_share_preview, parse_amount_text,
    participant_lookup_shares,
};
use crate::components::{
    ActionButton, ButtonTone, FieldBlock, ListRow, ModalSheet, ScreenFrame, SectionCard, TagPill,
    TopBarButton,
};
use leptos::prelude::*;

#[component]
pub fn StatusStrip(status: Option<String>, error: Option<String>, busy: bool) -> impl IntoView {
    let message = error.clone().or(status);
    let class_name = if message.is_some() {
        if error.is_some() {
            "status-strip status-strip-error"
        } else {
            "status-strip status-strip-info"
        }
    } else {
        "status-strip status-strip-hidden"
    };

    view! {
        <section class=class_name>
            <div class="status-copy">
                {message.unwrap_or_default()}
                {if busy {
                    view! { <span class="status-chip">"Working"</span> }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </section>
    }
}

#[component]
pub fn EmptyColumn(title: String, detail: String) -> impl IntoView {
    view! {
        <ScreenFrame title=title subtitle=detail>
            <SectionCard
                kicker="Ready".to_owned()
                title="Waiting for selection".to_owned()
                description="This column fills with the next page context.".to_owned()
            >
                <div class="empty-copy">
                    "Use the list on the left to open a ledger, bill, or settings view."
                </div>
            </SectionCard>
        </ScreenFrame>
    }
}

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
            subtitle="Available ledgers on this device".to_owned()
            trailing={view! { <TopBarButton label="More".to_owned() on_press=Callback::new(move |_| on_more.run(())) /> }.into_any()}
            footer={view! { <ActionButton label="New Ledger".to_owned() full_width=true on_press=Callback::new(move |_| on_new_ledger.run(())) /> }.into_any()}
        >
            <SectionCard
                kicker="Collection".to_owned()
                title="Ledgers".to_owned()
                description="Sorted by latest bill activity, then by name.".to_owned()
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
                                    meta=format!("{} members · {}", ledger.member_count, ledger.currency)
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
    let currency = detail.summary.currency.clone();

    view! {
        <ScreenFrame
            title=detail.summary.name
            subtitle="Bills in reverse time order".to_owned()
            leading={view! { <TopBarButton label="Back".to_owned() on_press=Callback::new(move |_| on_back.run(())) /> }.into_any()}
            trailing={view! { <TopBarButton label="More".to_owned() on_press=Callback::new(move |_| on_more.run(())) /> }.into_any()}
            footer={view! { <ActionButton label="New Bill".to_owned() full_width=true on_press=Callback::new(move |_| on_new_bill.run(())) /> }.into_any()}
        >
            <SectionCard
                kicker="Ledger".to_owned()
                title="Bills".to_owned()
                description="Each row opens amendment mode for the selected bill.".to_owned()
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
                                    meta=format!("Paid by {}", bill.payer_name)
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
pub fn DeviceSettingsPage(
    identities: Vec<Identity>,
    devices: Vec<SyncDevice>,
    on_back: Callback<()>,
    on_add_identity: Callback<()>,
    on_import_ledger: Callback<()>,
    on_scan_qr: Callback<()>,
    on_sync_device: Callback<String>,
) -> impl IntoView {
    view! {
        <ScreenFrame
            title="Device Settings".to_owned()
            subtitle="Local identities, known devices, and join actions".to_owned()
            leading={view! { <TopBarButton label="Back".to_owned() on_press=Callback::new(move |_| on_back.run(())) /> }.into_any()}
        >
            <div class="stack-gap">
                <SectionCard
                    kicker="Saved identities".to_owned()
                    title="On this device".to_owned()
                >
                    <div class="stack-gap">
                        {identities
                            .into_iter()
                            .map(|identity| {
                                view! { <ListRow title=identity.display_name meta=identity.user_id /> }
                            })
                            .collect_view()}
                    </div>
                </SectionCard>

                <SectionCard
                    kicker="Sync peers".to_owned()
                    title="Known devices".to_owned()
                    description="Authorized devices gathered from the ledgers stored on this device.".to_owned()
                >
                    <div class="stack-gap">
                        {if devices.is_empty() {
                            view! {
                                <div class="empty-copy">
                                    "No peer devices are available yet. Join a shared ledger to sync with another device."
                                </div>
                            }
                                .into_any()
                        } else {
                            devices
                                .into_iter()
                                .map(|device| {
                                    let node_id = device.node_id.clone();
                                    let detail = if device.ledger_names.is_empty() {
                                        "No shared ledgers".to_owned()
                                    } else {
                                        format!("Shared via {}", device.ledger_names.join(", "))
                                    };
                                    let title = if device.label.trim().is_empty() {
                                        "Unnamed device".to_owned()
                                    } else {
                                        device.label
                                    };
                                    view! {
                                        <div class="sync-device-row">
                                            <div class="row-copy">
                                                <p class="row-title">{title}</p>
                                                <p class="row-meta">{node_id.clone()}</p>
                                                <p class="row-detail">{detail}</p>
                                            </div>
                                            <ActionButton
                                                label="Sync".to_owned()
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

                <SectionCard
                    kicker="Actions".to_owned()
                    title="Import and add".to_owned()
                >
                    <div class="stack-gap">
                        <ActionButton
                            label="Add Identity".to_owned()
                            tone=ButtonTone::Secondary
                            full_width=true
                            on_press=Callback::new(move |_| on_add_identity.run(()))
                        />
                        <ActionButton
                            label="Import Ledger".to_owned()
                            tone=ButtonTone::Quiet
                            full_width=true
                            on_press=Callback::new(move |_| on_import_ledger.run(()))
                        />
                        <ActionButton
                            label="Join Ledger".to_owned()
                            tone=ButtonTone::Quiet
                            full_width=true
                            on_press=Callback::new(move |_| on_scan_qr.run(()))
                        />
                    </div>
                </SectionCard>
            </div>
        </ScreenFrame>
    }
}

#[component]
pub fn LedgerSettingsPage(
    detail: LedgerDetail,
    invitation_url: Option<String>,
    on_back: Callback<()>,
    on_add_member: Callback<()>,
    on_create_invitation: Callback<()>,
    on_copy_invitation: Callback<()>,
) -> impl IntoView {
    view! {
        <ScreenFrame
            title="Ledger Settings".to_owned()
            subtitle="Members and invitation flow".to_owned()
            leading={view! { <TopBarButton label="Back".to_owned() on_press=Callback::new(move |_| on_back.run(())) /> }.into_any()}
        >
            <div class="stack-gap">
                <SectionCard
                    kicker="Members".to_owned()
                    title=detail.summary.name.clone()
                >
                    <div class="stack-gap">
                        {detail
                            .members
                            .into_iter()
                            .map(|member| {
                                view! { <ListRow title=member.display_name meta=member.user_id /> }
                            })
                            .collect_view()}

                        <ActionButton
                            label="Add Member".to_owned()
                            tone=ButtonTone::Secondary
                            full_width=true
                            on_press=Callback::new(move |_| on_add_member.run(()))
                        />
                    </div>
                </SectionCard>

                <SectionCard
                    kicker="Invitation".to_owned()
                    title="Device invitation".to_owned()
                    description="Create a join URL and copy it onto another device.".to_owned()
                >
                    <div class="stack-gap">
                        <ActionButton
                            label="Device Invitation".to_owned()
                            tone=ButtonTone::Secondary
                            full_width=true
                            on_press=Callback::new(move |_| on_create_invitation.run(()))
                        />

                        {invitation_url
                            .map(|url| {
                                view! {
                                    <div class="invite-panel">
                                        <pre class="invite-url">{url.clone()}</pre>
                                        <ActionButton
                                            label="Copy URL".to_owned()
                                            tone=ButtonTone::Quiet
                                            full_width=true
                                            on_press=Callback::new(move |_| on_copy_invitation.run(()))
                                        />
                                    </div>
                                }
                                    .into_any()
                            })}
                    </div>
                </SectionCard>
            </div>
        </ScreenFrame>
    }
}

#[component]
pub fn BillEditorPage(
    title: String,
    currency: String,
    members: Vec<Member>,
    seed: BillEditorSeed,
    on_back: Callback<()>,
    on_save: Callback<BillSaveRequest>,
) -> impl IntoView {
    let description = RwSignal::new(seed.description);
    let amount_text = RwSignal::new(seed.amount_text);
    let payer_user_id = RwSignal::new(seed.payer_user_id.unwrap_or_default());
    let share_mode = RwSignal::new(seed.share_mode);
    let participants = RwSignal::new(seed.participants);
    let validation_error = RwSignal::new(None::<String>);
    let currency_field_value = currency.clone();
    let split_currency = currency.clone();

    let save_click = move |_| {
        let amount_cents = match parse_amount_text(&amount_text.get()) {
            Ok(value) => value,
            Err(error) => {
                validation_error.set(Some(error));
                return;
            }
        };

        let selected_payer = payer_user_id.get();
        if selected_payer.is_empty() {
            validation_error.set(Some("Choose a payer before saving.".to_owned()));
            return;
        }

        let active_participants = participants
            .get()
            .into_iter()
            .filter(|participant| participant.included)
            .collect::<Vec<_>>();

        if active_participants.is_empty() {
            validation_error.set(Some(
                "Select at least one participant before saving.".to_owned(),
            ));
            return;
        }

        let shares = active_participants
            .into_iter()
            .map(|participant| crate::api::BillShareInput {
                user_id: participant.user_id,
                shares: if share_mode.get() == ShareMode::Equal {
                    1
                } else {
                    participant.shares
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
            payer_user_id: selected_payer,
            amount_cents,
            shares,
        });
    };

    view! {
        <ScreenFrame
            title=title
            subtitle="Description, payer, amount, and participant shares".to_owned()
            leading={view! { <TopBarButton label="Back".to_owned() on_press=Callback::new(move |_| on_back.run(())) /> }.into_any()}
            trailing={view! { <ActionButton label="Save".to_owned() tone=ButtonTone::Secondary on_press=Callback::new(save_click) /> }.into_any()}
        >
            <div class="stack-gap">
                <SectionCard
                    kicker="Payment".to_owned()
                    title="Core fields".to_owned()
                    description="This form writes directly to the current bill model.".to_owned()
                >
                    <div class="stack-gap">
                        <FieldBlock label="Description".to_owned()>
                            <input
                                class="ui-input"
                                prop:value=move || description.get()
                                on:input=move |event| description.set(event_target_value(&event))
                            />
                        </FieldBlock>

                        <FieldBlock label="Payer".to_owned()>
                            <select
                                class="ui-select"
                                prop:value=move || payer_user_id.get()
                                on:change=move |event| payer_user_id.set(event_target_value(&event))
                            >
                                <option value="">"Select a member"</option>
                                {members
                                    .iter()
                                    .map(|member| {
                                        view! {
                                            <option value=member.user_id.clone()>{member.display_name.clone()}</option>
                                        }
                                    })
                                    .collect_view()}
                            </select>
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
                    kicker="Participants".to_owned()
                    title="Share split".to_owned()
                    description="Equal split assigns one share per active participant.".to_owned()
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
                            let current_rows = participants.get();
                            let preview = derived_share_preview(current_amount, current_mode, &current_rows);

                            current_rows
                                .into_iter()
                                .map(|participant| {
                                    let participant_id = participant.user_id.clone();
                                    let toggle_participant_id = participant_id.clone();
                                    let share_participant_id = participant_id.clone();
                                    let share_value_id = participant_id.clone();
                                    let display_name = participant.display_name.clone();
                                    let preview_text = preview
                                        .iter()
                                        .find(|(user_id, _)| user_id == &participant_id)
                                        .map(|(_, cents)| api::format_money(*cents, &split_currency))
                                        .unwrap_or_else(|| format!("{} 0.00", split_currency));

                                    view! {
                                        <div class="participant-row">
                                            <label class="participant-toggle">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=participant.included
                                                    on:change=move |event| {
                                                        let checked = event_target_checked(&event);
                                                        participants.update(|items| {
                                                            if let Some(item) = items.iter_mut().find(|item| item.user_id == toggle_participant_id) {
                                                                item.included = checked;
                                                            }
                                                        });
                                                    }
                                                />
                                                <span>{display_name}</span>
                                            </label>

                                            <div class="participant-side">
                                                {if current_mode == ShareMode::Custom {
                                                    view! {
                                                        <input
                                                            class="participant-share-input"
                                                            prop:value=participant_lookup_shares(&participants.get(), &share_value_id).to_string()
                                                            on:input=move |event| {
                                                                let value = event_target_value(&event)
                                                                    .parse::<u32>()
                                                                    .ok()
                                                                    .filter(|value| *value > 0)
                                                                    .unwrap_or(1);
                                                                participants.update(|items| {
                                                                    if let Some(item) = items.iter_mut().find(|item| item.user_id == share_participant_id) {
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
                                                <span class="participant-amount">{preview_text}</span>
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
            description="Name the ledger and choose its currency.".to_owned()
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
                    <select
                        class="ui-select"
                        prop:value=move || currency.get()
                        on:change=move |event| currency.set(event_target_value(&event))
                    >
                        <option value="USD">"USD"</option>
                        <option value="EUR">"EUR"</option>
                        <option value="GBP">"GBP"</option>
                    </select>
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
pub fn AddIdentitySheet(on_cancel: Callback<()>, on_submit: Callback<String>) -> impl IntoView {
    let display_name = RwSignal::new(String::new());

    view! {
        <ModalSheet
            title="Add Identity".to_owned()
            description="Save a person on this device for later reuse.".to_owned()
            on_close=Callback::new(move |_| on_cancel.run(()))
        >
            <div class="stack-gap">
                <FieldBlock label="Identity name".to_owned()>
                    <input
                        class="ui-input"
                        prop:value=move || display_name.get()
                        on:input=move |event| display_name.set(event_target_value(&event))
                    />
                </FieldBlock>
                <ActionButton
                    label="Save Identity".to_owned()
                    full_width=true
                    on_press=Callback::new(move |_| on_submit.run(display_name.get()))
                />
            </div>
        </ModalSheet>
    }
}

#[component]
pub fn AddMemberSheet(on_cancel: Callback<()>, on_submit: Callback<String>) -> impl IntoView {
    let display_name = RwSignal::new(String::new());

    view! {
        <ModalSheet
            title="Add Member".to_owned()
            description="Append a member to the current ledger.".to_owned()
            on_close=Callback::new(move |_| on_cancel.run(()))
        >
            <div class="stack-gap">
                <FieldBlock label="Member name".to_owned()>
                    <input
                        class="ui-input"
                        prop:value=move || display_name.get()
                        on:input=move |event| display_name.set(event_target_value(&event))
                    />
                </FieldBlock>
                <ActionButton
                    label="Add Member".to_owned()
                    full_width=true
                    on_press=Callback::new(move |_| on_submit.run(display_name.get()))
                />
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
            description="Confirm the invitation URL and enter the device label to store in the ledger.".to_owned()
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
                <FieldBlock label="Device label".to_owned()>
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
