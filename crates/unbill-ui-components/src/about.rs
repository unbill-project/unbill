use crate::button::{ActionButton, ButtonTone, IconButton, IconButtonKind};
use leptos::prelude::*;

pub type AboutLinkRequest = (String, Callback<Result<(), String>>);

// sirno:witness:about-page:begin
/// Shared offline page. Hosts may handle links in their system browser.
#[component]
pub fn OpenAbout(
    #[prop(optional)] on_open_link: Option<Callback<AboutLinkRequest>>,
) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let link_error = RwSignal::new(None::<String>);
    let about = unbill_about::content();
    let close = Callback::new(move |(): ()| {
        if let Some(dialog) = dialog.get() {
            dialog.close();
        }
    });
    view! {
        <ActionButton label=about.title.clone() tone=ButtonTone::Quiet
            on_press=Callback::new(move |_| {
                link_error.set(None);
                if let Some(dialog) = dialog.get() {
                    let _ = dialog.show_modal();
                }
            }) />
        // A native modal dialog supplies focus trapping, Escape, and focus return.
        <dialog node_ref=dialog class="about-dialog" aria-label=about.title.clone()
            on:click=move |event| {
                if event.target() == event.current_target() { close.run(()); }
            }>
            <article class="about-shell">
                <header class="sheet-header">
                    <h2 class="sheet-title">{about.title.clone()}</h2>
                    <IconButton kind=IconButtonKind::Close on_press=Callback::new(move |_| close.run(())) />
                </header>
                <div class="about-scroll">
                    <header class="about-hero">
                        <div class="about-brand">
                            <span class="about-mark" aria-hidden="true"><i></i><i></i><i></i></span>
                            <span class="about-wordmark">{about.name.clone()}</span>
                        </div>
                        <h3>{about.tagline.clone()}</h3>
                        <p class="about-summary">{about.summary.clone()}</p>
                        <ul class="about-principles">
                            {about.principles.iter().map(|label| view! { <li>{label.clone()}</li> }).collect_view()}
                        </ul>
                    </header>
                    {about.sections.iter().map(|section| {
                        let section_id = section.id.clone();
                        view! {
                            <section class="about-section" class:about-support=section.id == "support">
                                <h3>{section.title.clone()}</h3>
                                {section.paragraphs.iter().map(|paragraph| view! { <p>{paragraph.clone()}</p> }).collect_view()}
                                {section.link.as_ref().map(|link| view! {
                                    <a class="about-link" href=link.url.clone() target="_blank" rel="noopener noreferrer"
                                        on:click=move |event| {
                                            if let Some(open) = on_open_link {
                                                event.prevent_default();
                                                link_error.set(None);
                                                open.run((section_id.clone(), Callback::new(move |result: Result<(), String>| {
                                                    link_error.set(result.err());
                                                })));
                                            }
                                        }>
                                        <span>{link.label.clone()}</span>
                                        <span aria-hidden="true">{if section.id == "support" { "☆" } else { "↗" }}</span>
                                    </a>
                                })}
                            </section>
                        }
                    }).collect_view()}
                </div>
                {move || link_error.get().map(|error| view! {
                    <p class="form-error about-link-error" role="alert">{error}</p>
                })}
            </article>
        </dialog>
    }
}
// sirno:witness:about-page:end
