use iso_currency::IntoEnumIterator;
use leptos::prelude::*;
use unbill_model::validation;

#[component]
pub fn TextInput(
    #[prop(into)] value: Signal<String>,
    on_change: Callback<String>,
    #[prop(into, default = String::new())] placeholder: String,
    #[prop(default = false)] disabled: bool,
) -> impl IntoView {
    view! {
        <input
            type="text"
            prop:value=value
            placeholder=placeholder
            disabled=disabled
            on:input=move |ev| on_change.run(event_target_value(&ev))
        />
    }
}

#[component]
pub fn AmountInput(
    /// Value in cents.
    #[prop(into)]
    value_cents: Signal<i64>,
    on_change: Callback<i64>,
    #[prop(default = false)] disabled: bool,
) -> impl IntoView {
    let display = move || {
        let cents = value_cents.get();
        format!("{:.2}", cents as f64 / 100.0)
    };

    view! {
        <input
            type="number"
            step="0.01"
            prop:value=display
            disabled=disabled
            on:input=move |ev| {
                let raw = event_target_value(&ev);
                if let Ok(cents) = validation::parse_bill_amount_text(&raw) {
                    on_change.run(cents);
                }
            }
        />
    }
}

#[component]
pub fn CurrencyCombobox(value: RwSignal<String>) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let open = RwSignal::new(false);
    let dropdown_style = RwSignal::new(String::new());
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let display = Memo::new(move |_| value.get());

    let filtered = Memo::new(move |_| {
        let q = query.get().to_lowercase();
        iso_currency::Currency::iter()
            .filter(move |c| {
                q.is_empty()
                    || c.code().to_lowercase().contains(&q)
                    || c.name().to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    });

    view! {
        <div class="combobox">
            <input
                node_ref=input_ref
                class="ui-input"
                prop:value=move || if open.get() { query.get() } else { display.get() }
                placeholder=move || if open.get() { display.get() } else { String::new() }
                on:blur=move |_| {
                    open.set(false);
                    query.set(String::new());
                }
                on:input=move |event| {
                    let q = event_target_value(&event);
                    query.set(q);
                    if !open.get() {
                        if let Some(el) = input_ref.get() {
                            let rect = el.get_bounding_client_rect();
                            dropdown_style.set(format!(
                                "top:{}px;left:{}px;width:{}px",
                                rect.bottom() + 4.0,
                                rect.left(),
                                rect.width(),
                            ));
                        }
                        open.set(true);
                    }
                }
            />
            <Show when=move || open.get()>
                <ul class="combobox-list" style=move || dropdown_style.get()>
                    {move || {
                        filtered
                            .get()
                            .into_iter()
                            .map(|c| {
                                let code = c.code();
                                let label = format!("{} — {}", code, c.name());
                                view! {
                                    <li
                                        class="combobox-option"
                                        on:mousedown=move |event| {
                                            event.prevent_default();
                                            value.set(code.to_owned());
                                            query.set(String::new());
                                            open.set(false);
                                        }
                                    >
                                        {label}
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </Show>
        </div>
    }
}
