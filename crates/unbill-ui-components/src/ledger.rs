use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::layout::{FieldBlock, ModalSheet};

pub type RenameLedgerRequest = (String, Callback<Result<(), String>>);

// sirno:witness:ledger-rename:begin
#[component]
pub fn RenameLedgerSheet(
    current_name: String,
    on_cancel: Callback<()>,
    on_saved: Callback<()>,
    on_submit: Callback<RenameLedgerRequest>,
) -> impl IntoView {
    let name = RwSignal::new(current_name.clone());
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let valid = move |value: &str| !value.is_empty() && value.len() <= 100 && value != current_name;
    let valid_submit = valid.clone();

    view! {
        <ModalSheet
            title="Rename Ledger".to_owned()
            description="The name is shared with other devices when the ledger syncs.".to_owned()
            on_close=Callback::new(move |_| { if !saving.get_untracked() { on_cancel.run(()); } })
        >
            <form class="stack-gap" on:submit={
                move |event: leptos::ev::SubmitEvent| {
                    event.prevent_default();
                    let value = name.get_untracked().trim().to_owned();
                    if saving.get_untracked() || !valid_submit(&value) {
                        return;
                    }
                    saving.set(true);
                    error.set(None);
                    on_submit.run((value, Callback::new(move |result| {
                        saving.set(false);
                        match result {
                            Ok(()) => on_saved.run(()),
                            Err(message) => error.set(Some(message)),
                        }
                    })));
                }
            }>
                <FieldBlock label="Ledger name".to_owned()>
                    <input
                        aria-label="Ledger name"
                        class="ui-input"
                        prop:value=move || name.get()
                        disabled=move || saving.get()
                        on:input=move |event| name.set(event_target_value(&event))
                    />
                </FieldBlock>
                {move || (name.get().trim().len() > 100).then(|| view! {
                    <p class="form-error">"Ledger names can be at most 100 UTF-8 bytes."</p>
                })}
                {move || error.get().map(|message| view! { <p class="form-error" role="alert">{message}</p> })}
                <button
                    type="submit"
                    class="action-button action-button-primary action-button-block"
                    disabled=move || saving.get() || !valid(name.get().trim())
                >"Save"</button>
            </form>
        </ModalSheet>
    }
}
// sirno:witness:ledger-rename:end

// sirno:witness:ui-components:begin
#[component]
pub fn LedgerFingerprint(fingerprint: String, on_copy: Callback<String>) -> impl IntoView {
    let copy_text = fingerprint.clone();
    view! {
        <div class="stack-gap">
            <p class="row-title">"Ledger fingerprint"</p>
            <p style="display:flex;flex-wrap:wrap;gap:0.25rem;font-size:1.35rem;line-height:1.6;min-width:0;user-select:text;">
                {fingerprint.split(' ').map(|symbol| view! {
                    <span style="white-space:nowrap;">{format!("{symbol} ")}</span>
                }).collect_view()}
            </p>
            <p class="row-meta">"Matching fingerprints suggest these devices have the same ledger state."</p>
            <crate::button::ActionButton
                label="Copy fingerprint".to_owned()
                tone=crate::button::ButtonTone::Secondary
                on_press=Callback::new(move |_| on_copy.run(copy_text.clone()))
            />
        </div>
    }
}
// sirno:witness:ui-components:end

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LedgerItem {
    pub id: String,
    pub name: String,
    pub currency_code: String,
}

#[component]
pub fn LedgerRow(ledger: LedgerItem, on_tap: Callback<String>) -> impl IntoView {
    let id = ledger.id.clone();
    view! {
        <div
            class="ledger-row"
            on:click=move |_| on_tap.run(id.clone())
        >
            <span class="ledger-name">{ledger.name.clone()}</span>
            <span class="ledger-currency">{ledger.currency_code.clone()}</span>
        </div>
    }
}

#[component]
pub fn LedgerList(
    #[prop(into)] ledgers: Signal<Vec<LedgerItem>>,
    on_tap: Callback<String>,
    #[prop(optional)] on_refresh: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <div class="ledger-list">
            {move || ledgers.get().into_iter().map(|ledger| {
                view! { <LedgerRow ledger=ledger.clone() on_tap=on_tap /> }
            }).collect_view()}
            {on_refresh.map(|cb| view! {
                <button on:click=move |_| cb.run(())>"Refresh"</button>
            })}
        </div>
    }
}
