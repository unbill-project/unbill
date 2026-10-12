use crate::button::{ActionButton, ButtonTone, IconButton, IconButtonKind};
use leptos::prelude::*;

fn matches_search(name: &str, query: &str) -> bool {
    name.to_ascii_lowercase()
        .contains(&query.trim().to_ascii_lowercase())
}

// sirno:witness:unbill-license:begin
/// Local, on-demand license browser shared by Tauri and browser settings.
#[component]
pub fn OpenSourceLicenses() -> impl IntoView {
    let open = RwSignal::new(false);
    view! {
        <ActionButton label="Open Source Licenses".to_owned() tone=ButtonTone::Quiet
            on_press=Callback::new(move |_| open.set(true)) />
        <Show when=move || open.get()>
            <LicenseBrowser on_close=Callback::new(move |_| open.set(false)) />
        </Show>
    }
}

#[component]
fn LicenseBrowser(on_close: Callback<()>) -> impl IntoView {
    let summaries = StoredValue::new(unbill_license::software_summaries());
    let query = RwSignal::new(String::new());
    let selected = RwSignal::new("unbill".to_owned());
    let filtered = Memo::new(move |_| {
        let search = query.get();
        summaries.with_value(|items| {
            items
                .iter()
                .filter(|item| matches_search(&item.name, &search))
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    view! {
        <div class="sheet-overlay license-overlay" on:keydown=move |event| {
            if event.key() == "Escape" { on_close.run(()); }
        }>
            <div class="sheet-backdrop" on:click=move |_| on_close.run(())></div>
            <section class="sheet-panel license-panel" role="dialog" aria-modal="true" aria-label="Open Source Licenses">
                <header class="sheet-header">
                    <h2 class="sheet-title">"Open Source Licenses"</h2>
                    <IconButton kind=IconButtonKind::Close on_press=Callback::new(move |_| on_close.run(())) />
                </header>
                <p class="row-meta license-scope">"Unbill and third-party Rust software"</p>
                <div class="license-layout">
                    <aside class="license-sidebar">
                        <input class="ui-input" type="search" placeholder="Search software"
                            aria-label="Search software" autofocus
                            prop:value=move || query.get()
                            on:input=move |event| query.set(event_target_value(&event)) />
                        <div class="license-software-list" role="group" aria-label="Software">
                            {move || if filtered.get().is_empty() {
                                view! { <p class="empty-copy">"No matching software."</p> }.into_any()
                            } else {
                                filtered.get().into_iter().map(|item| {
                                    let name = item.name.clone();
                                    let selected_name = name.clone();
                                    let title = if name == "unbill" { "Unbill".to_owned() } else { name.clone() };
                                    view! {
                                        <button type="button" class="license-software" aria-pressed=move || selected.get() == selected_name
                                            on:click=move |_| selected.set(name.clone())>
                                            <span>{title}</span>
                                            <small>{item.versions.join(", ")}</small>
                                        </button>
                                    }
                                }).collect_view().into_any()
                            }}
                        </div>
                    </aside>
                    <div class="license-documents" role="region" aria-label="License texts">
                        {move || unbill_license::software(&selected.get()).iter().map(|package| {
                            let title = if package.name == "unbill" { "Unbill" } else { &package.name };
                            let documents = unbill_license::display_documents(package);
                            view! {
                                <section class="license-package">
                                    <h3>{format!("{} {}", title, package.version)}</h3>
                                    <p>{package.license_expression.clone()}</p>
                                    {package.repository.clone().map(|source| view! { <p class="row-meta license-source">{source}</p> })}
                                    {package.source_archive.clone().or_else(|| package.source.clone()).map(|source| view! { <p class="row-meta license-source">{source}</p> })}
                                    {documents.into_iter().map(|document| view! {
                                        <section class="license-document">
                                            <h4>{document.path}</h4>
                                            <pre>{document.text}</pre>
                                        </section>
                                    }).collect_view()}
                                </section>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </section>
        </div>
    }
}
// sirno:witness:unbill-license:end

#[cfg(test)]
mod tests {
    use super::matches_search;

    #[test]
    fn software_search_ignores_case_and_surrounding_whitespace() {
        assert!(matches_search("Unbill", " UNB "));
        assert!(matches_search("iroh", ""));
        assert!(!matches_search("serde", "iroh"));
    }
}
