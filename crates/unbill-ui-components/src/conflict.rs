use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictBillItem {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub detail: String,
}

#[component]
pub fn ConflictGroupView(
    index: usize,
    bills: Vec<ConflictBillItem>,
    ancestor_count: usize,
    on_commit: Callback<(String, Vec<String>)>,
) -> impl IntoView {
    let selected_bill_id = RwSignal::new(None::<String>);
    let bill_ids = bills.iter().map(|bill| bill.id.clone()).collect::<Vec<_>>();
    let commit_ids = bill_ids.clone();
    let commit_selection = move |_| {
        if let Some(selected) = selected_bill_id.get() {
            on_commit.run((selected, commit_ids.clone()));
        }
    };

    view! {
        <div class="conflict-group">
            <div class="conflict-summary">
                <p class="row-title">{format!("Conflict {}", index + 1)}</p>
                <p class="row-meta">{format!("{} competing versions", bill_ids.len())}</p>
            </div>
            <div class="stack-gap">
                {bills
                    .into_iter()
                    .map(|bill| {
                        let bill_id = bill.id.clone();
                        let select_id = bill.id.clone();
                        view! {
                            <ConflictBillRow
                                bill=bill
                                selected=Signal::derive(move || selected_bill_id.get().as_deref() == Some(bill_id.as_str()))
                                on_select=Callback::new(move |_| selected_bill_id.set(Some(select_id.clone())))
                            />
                        }
                    })
                    .collect_view()}
            </div>
            {if ancestor_count == 0 {
                view! { <div /> }.into_any()
            } else {
                view! {
                    <p class="conflict-ancestor-count">
                        {format!("{} superseded ancestor{}", ancestor_count, if ancestor_count == 1 { "" } else { "s" })}
                    </p>
                }
                .into_any()
            }}
            <div class="conflict-actions">
                <button
                    type="button"
                    class="action-button action-button-secondary"
                    disabled=move || selected_bill_id.get().is_none()
                    on:click=commit_selection
                >
                    "Commit Selection"
                </button>
            </div>
        </div>
    }
}

#[component]
fn ConflictBillRow(
    bill: ConflictBillItem,
    #[prop(into)] selected: Signal<bool>,
    on_select: Callback<()>,
) -> impl IntoView {
    view! {
        <div class=move || if selected.get() { "conflict-version conflict-version-selected" } else { "conflict-version" }>
            <div class="row-copy">
                <p class="row-title">{bill.title}</p>
                <p class="row-meta">{bill.meta}</p>
                <p class="row-detail">{bill.detail}</p>
            </div>
            <button
                type="button"
                class=move || if selected.get() { "action-button action-button-primary" } else { "action-button action-button-quiet" }
                on:click=move |_| on_select.run(())
            >
                {move || if selected.get() { "Selected" } else { "Use This" }}
            </button>
        </div>
    }
}
