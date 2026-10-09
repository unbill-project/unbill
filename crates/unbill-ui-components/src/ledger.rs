use leptos::prelude::*;
use serde::{Deserialize, Serialize};

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
