use std::collections::HashMap;

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::button::{ButtonTone, IconButton, IconButtonKind, TagPill};
use crate::layout::{FieldBlock, ScreenFrame, SectionCard};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShareMode {
    Equal,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillEditorUser {
    pub user_id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillShareDraft {
    pub user_id: String,
    pub display_name: String,
    pub included: bool,
    pub shares: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillShareInput {
    pub user_id: String,
    pub shares: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillEditorExistingBill {
    pub id: String,
    pub amount_cents: i64,
    pub description: String,
    pub payers: Vec<BillShareInput>,
    pub payees: Vec<BillShareInput>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillEditorSeed {
    pub currency: String,
    pub prev_bill_id: Option<String>,
    pub description: String,
    pub payer_mode: ShareMode,
    pub payer_rows: Vec<BillShareDraft>,
    pub amount_text: String,
    pub share_mode: ShareMode,
    pub share_rows: Vec<BillShareDraft>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillSaveRequest {
    pub prev_bill_id: Option<String>,
    pub description: String,
    pub payers: Vec<BillShareInput>,
    pub amount_cents: i64,
    pub shares: Vec<BillShareInput>,
}

pub fn new_bill_seed(currency: String, users: &[BillEditorUser]) -> BillEditorSeed {
    BillEditorSeed {
        currency,
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
                shares: "1".to_owned(),
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
                shares: "1".to_owned(),
            })
            .collect(),
    }
}

pub fn amend_bill_seed(
    bill: &BillEditorExistingBill,
    currency: String,
    users: &[BillEditorUser],
) -> BillEditorSeed {
    let payers_by_user = bill
        .payers
        .iter()
        .map(|share| (share.user_id.clone(), share.shares))
        .collect::<HashMap<_, _>>();
    let payees_by_user = bill
        .payees
        .iter()
        .map(|share| (share.user_id.clone(), share.shares))
        .collect::<HashMap<_, _>>();

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
        prev_bill_id: Some(bill.id.clone()),
        description: bill.description.clone(),
        payer_mode,
        payer_rows: users
            .iter()
            .map(|user| BillShareDraft {
                user_id: user.user_id.clone(),
                display_name: user.display_name.clone(),
                included: payers_by_user.contains_key(&user.user_id),
                shares: payers_by_user
                    .get(&user.user_id)
                    .copied()
                    .unwrap_or(1)
                    .to_string(),
            })
            .collect(),
        amount_text: format_cents_for_input(bill.amount_cents),
        share_mode,
        share_rows: users
            .iter()
            .map(|user| BillShareDraft {
                user_id: user.user_id.clone(),
                display_name: user.display_name.clone(),
                included: payees_by_user.contains_key(&user.user_id),
                shares: payees_by_user
                    .get(&user.user_id)
                    .copied()
                    .unwrap_or(1)
                    .to_string(),
            })
            .collect(),
    }
}

pub fn parse_amount_text(input: &str) -> Result<i64, String> {
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

pub fn build_bill_save_request(
    prev_bill_id: Option<String>,
    description: String,
    payer_mode: ShareMode,
    payer_rows: &[BillShareDraft],
    amount_text: &str,
    share_mode: ShareMode,
    share_rows: &[BillShareDraft],
) -> Result<BillSaveRequest, String> {
    let amount_cents = parse_amount_text(amount_text)?;

    let active_payer_rows = payer_rows
        .iter()
        .filter(|row| row.included)
        .collect::<Vec<_>>();

    if active_payer_rows.is_empty() {
        return Err("Select at least one payer before saving.".to_owned());
    }

    let payers = active_payer_rows
        .into_iter()
        .map(|row| {
            let shares = if payer_mode == ShareMode::Equal {
                1
            } else {
                parse_required_share_weight(
                    &row.shares,
                    "Custom payer shares must be positive whole numbers.",
                )?
            };

            Ok(BillShareInput {
                user_id: row.user_id.clone(),
                shares,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let active_share_rows = share_rows
        .iter()
        .filter(|share_row| share_row.included)
        .collect::<Vec<_>>();

    if active_share_rows.is_empty() {
        return Err("Select at least one payee before saving.".to_owned());
    }

    let shares = active_share_rows
        .into_iter()
        .map(|share_row| {
            let shares = if share_mode == ShareMode::Equal {
                1
            } else {
                parse_required_share_weight(
                    &share_row.shares,
                    "Custom shares must be positive whole numbers.",
                )?
            };

            Ok(BillShareInput {
                user_id: share_row.user_id.clone(),
                shares,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(BillSaveRequest {
        prev_bill_id,
        description,
        payers,
        amount_cents,
        shares,
    })
}

pub fn share_lookup_shares(share_rows: &[BillShareDraft], user_id: &str) -> String {
    share_rows
        .iter()
        .find(|share_row| share_row.user_id == user_id)
        .map(|share_row| share_row.shares.clone())
        .unwrap_or_else(|| "1".to_owned())
}

pub fn derived_share_preview(
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
                    parse_share_weight(&share_row.shares).unwrap_or(0)
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

#[component]
pub fn BillEditorPage(
    title: String,
    currency: String,
    seed: BillEditorSeed,
    #[prop(optional)] show_back: bool,
    on_back: Callback<()>,
    on_save: Callback<BillSaveRequest>,
) -> impl IntoView {
    let prev_bill_id = seed.prev_bill_id.clone();
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
        let current_payer_rows = payer_rows.get();
        let current_share_rows = share_rows.get();
        let current_amount_text = amount_text.get();

        match build_bill_save_request(
            prev_bill_id.clone(),
            description.get(),
            payer_mode.get(),
            &current_payer_rows,
            &current_amount_text,
            share_mode.get(),
            &current_share_rows,
        ) {
            Ok(request) => {
                validation_error.set(None);
                on_save.run(request);
            }
            Err(error) => validation_error.set(Some(error)),
        }
    };

    view! {
        <ScreenFrame
            header={view! {
                <div class="screen-leading">
                    {show_back.then(|| view! { <IconButton kind=IconButtonKind::Back on_press=Callback::new(move |_| on_back.run(())) /> })}
                </div>
                <div class="screen-copy">
                    <h2 class="screen-title">{title}</h2>
                    <p class="screen-subtitle">"Bill details"</p>
                </div>
                <div class="screen-trailing">
                    <IconButton kind=IconButtonKind::Save tone=ButtonTone::Secondary on_press=Callback::new(save_click) />
                </div>
            }.into_any()}
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
                                        .map(|(_, cents)| format_money(*cents, &payer_currency))
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
                                                                let value = event_target_value(&event);
                                                                payer_rows.update(|items| {
                                                                    if let Some(item) = items.iter_mut().find(|item| item.user_id == share_user_id) {
                                                                        item.shares = value.clone();
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
                                        .map(|(_, cents)| format_money(*cents, &split_currency))
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
                                                                let value = event_target_value(&event);
                                                                share_rows.update(|items| {
                                                                    if let Some(item) = items.iter_mut().find(|item| item.user_id == share_user_id) {
                                                                        item.shares = value.clone();
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

fn parse_required_share_weight(input: &str, error: &str) -> Result<u32, String> {
    parse_share_weight(input).ok_or_else(|| error.to_owned())
}

fn parse_share_weight(input: &str) -> Option<u32> {
    input
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|shares| *shares > 0)
}

fn format_cents_for_input(amount_cents: i64) -> String {
    format!("{}.{:02}", amount_cents / 100, amount_cents.abs() % 100)
}

fn format_money(amount_cents: i64, currency: &str) -> String {
    let sign = if amount_cents < 0 { "-" } else { "" };
    let absolute = amount_cents.abs();
    format!("{sign}{currency} {}.{:02}", absolute / 100, absolute % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(user_id: &str, display_name: &str) -> BillEditorUser {
        BillEditorUser {
            user_id: user_id.to_owned(),
            display_name: display_name.to_owned(),
        }
    }

    fn bill_share_draft(user_id: &str, included: bool, shares: &str) -> BillShareDraft {
        BillShareDraft {
            user_id: user_id.to_owned(),
            display_name: user_id.to_owned(),
            included,
            shares: shares.to_owned(),
        }
    }

    #[test]
    fn new_bill_seed_selects_first_payer_and_all_payees() {
        let seed = new_bill_seed(
            "USD".to_owned(),
            &[user("alice", "Alice"), user("bob", "Bob")],
        );

        assert_eq!(seed.prev_bill_id, None);
        assert_eq!(seed.payer_mode, ShareMode::Equal);
        assert_eq!(seed.share_mode, ShareMode::Equal);
        assert_eq!(
            seed.payer_rows
                .iter()
                .map(|row| (row.user_id.as_str(), row.included, row.shares.as_str()))
                .collect::<Vec<_>>(),
            vec![("alice", true, "1"), ("bob", false, "1")]
        );
        assert!(seed.share_rows.iter().all(|row| row.included));
    }

    #[test]
    fn amend_bill_seed_preserves_custom_participants_and_share_modes() {
        let bill = BillEditorExistingBill {
            id: "bill-1".to_owned(),
            amount_cents: 1234,
            description: "Dinner".to_owned(),
            payers: vec![BillShareInput {
                user_id: "alice".to_owned(),
                shares: 2,
            }],
            payees: vec![
                BillShareInput {
                    user_id: "alice".to_owned(),
                    shares: 1,
                },
                BillShareInput {
                    user_id: "bob".to_owned(),
                    shares: 3,
                },
            ],
        };

        let seed = amend_bill_seed(
            &bill,
            "USD".to_owned(),
            &[user("alice", "Alice"), user("bob", "Bob")],
        );

        assert_eq!(seed.prev_bill_id.as_deref(), Some("bill-1"));
        assert_eq!(seed.amount_text, "12.34");
        assert_eq!(seed.payer_mode, ShareMode::Custom);
        assert_eq!(seed.share_mode, ShareMode::Custom);
        assert_eq!(seed.payer_rows[0].shares, "2");
        assert_eq!(seed.share_rows[1].shares, "3");
    }

    #[test]
    fn custom_share_drafts_keep_raw_text_before_submission() {
        let rows = vec![
            bill_share_draft("alice", true, ""),
            bill_share_draft("bob", true, "2"),
        ];

        assert_eq!(share_lookup_shares(&rows, "alice"), "");
        assert_eq!(
            derived_share_preview(900, ShareMode::Custom, &rows),
            vec![("alice".to_owned(), 0), ("bob".to_owned(), 900)]
        );
    }

    #[test]
    fn bill_save_request_rejects_invalid_custom_payer_shares_when_submitted() {
        let payer_rows = vec![bill_share_draft("alice", true, "")];
        let share_rows = vec![bill_share_draft("alice", true, "1")];

        let result = build_bill_save_request(
            None,
            "Coffee".to_owned(),
            ShareMode::Custom,
            &payer_rows,
            "12.00",
            ShareMode::Equal,
            &share_rows,
        );

        let error = match result {
            Ok(_) => panic!("invalid custom payer shares should fail on submission"),
            Err(error) => error,
        };
        assert_eq!(error, "Custom payer shares must be positive whole numbers.");
    }

    #[test]
    fn bill_save_request_builds_with_valid_custom_shares_on_submission() {
        let payer_rows = vec![
            bill_share_draft("alice", true, "2"),
            bill_share_draft("bob", false, "not validated while excluded"),
        ];
        let share_rows = vec![
            bill_share_draft("alice", true, "1"),
            bill_share_draft("bob", true, "3"),
        ];

        let request = build_bill_save_request(
            Some("previous-bill".to_owned()),
            "Dinner".to_owned(),
            ShareMode::Custom,
            &payer_rows,
            "12.30",
            ShareMode::Custom,
            &share_rows,
        )
        .expect("valid custom shares should submit");

        assert_eq!(request.prev_bill_id.as_deref(), Some("previous-bill"));
        assert_eq!(request.description, "Dinner");
        assert_eq!(request.amount_cents, 1230);
        assert_eq!(
            request.payers,
            vec![BillShareInput {
                user_id: "alice".to_owned(),
                shares: 2,
            }]
        );
        assert_eq!(
            request.shares,
            vec![
                BillShareInput {
                    user_id: "alice".to_owned(),
                    shares: 1,
                },
                BillShareInput {
                    user_id: "bob".to_owned(),
                    shares: 3,
                },
            ]
        );
    }
}
