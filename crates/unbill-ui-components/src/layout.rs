use leptos::{ev, prelude::*};

use crate::button::{IconButton, IconButtonKind};

#[component]
pub fn EmptyColumn(detail: String) -> impl IntoView {
    view! {
        <div class="empty-column">
            <p class="empty-copy">{detail}</p>
        </div>
    }
}

// sirno:witness:ui-components:begin
#[component]
pub fn Page(children: Children) -> impl IntoView {
    view! {
        <div class="page">
            {children()}
        </div>
    }
}

#[component]
pub fn SafeAreaContainer(children: Children) -> impl IntoView {
    view! {
        <div class="safe-area-container">
            {children()}
        </div>
    }
}

/// A bottom sheet / drawer overlay.
///
/// The wrapper is always in the DOM so CSS transitions can animate it in/out.
/// Add a `hidden` style to `.sheet-wrapper.hidden` in your stylesheet.
#[component]
pub fn Sheet(
    #[prop(into)] open: Signal<bool>,
    on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    let children = children();
    view! {
        <div class="sheet-wrapper" class:hidden=move || !open.get()>
            <div class="sheet-backdrop" on:click=move |_| on_close.run(())></div>
            <div class="sheet">
                {children}
            </div>
        </div>
    }
}

#[component]
pub fn ScreenFrame(
    #[prop(optional)] header: Option<AnyView>,
    children: Children,
    #[prop(optional)] footer: Option<AnyView>,
) -> impl IntoView {
    let header_view =
        header.map(|content| view! { <header class="screen-topbar">{content}</header> }.into_any());
    let footer_view =
        footer.map(|content| view! { <footer class="screen-footer">{content}</footer> }.into_any());

    view! {
        <section class="screen-frame">
            {header_view}
            <div class="screen-content">{children()}</div>
            {footer_view}
        </section>
    }
}

#[component]
pub fn ModalSheet(
    #[prop(into)] title: String,
    #[prop(optional, into)] description: Option<String>,
    #[prop(optional)] on_close: Option<Callback<ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    let description_view =
        description.map(|text| view! { <p class="sheet-description">{text}</p> }.into_any());

    view! {
        <div class="sheet-overlay">
            <div class="sheet-backdrop"></div>
            <section class="sheet-panel">
                <header class="sheet-header">
                    <div class="sheet-copy">
                        <p class="section-kicker">"Action"</p>
                        <h3 class="sheet-title">{title}</h3>
                        {description_view}
                    </div>
                    <IconButton
                        kind=IconButtonKind::Close
                        on_press=Callback::new(move |event| {
                            if let Some(handler) = on_close.as_ref() {
                                handler.run(event);
                            }
                        })
                    />
                </header>
                <div class="sheet-body">{children()}</div>
            </section>
        </div>
    }
}

#[component]
pub fn SectionCard(
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] kicker: Option<String>,
    #[prop(optional, into)] description: Option<String>,
    children: Children,
) -> impl IntoView {
    let kicker_view = kicker.map(|text| view! { <p class="section-kicker">{text}</p> }.into_any());
    let title_view = title.map(|text| view! { <h3 class="section-title">{text}</h3> }.into_any());
    let description_view =
        description.map(|text| view! { <p class="section-description">{text}</p> }.into_any());

    view! {
        <section class="section-card">
            <div class="section-header">
                {kicker_view}
                {title_view}
                {description_view}
            </div>
            <div class="section-body">{children()}</div>
        </section>
    }
}
// sirno:witness:ui-components:end

#[component]
pub fn FieldBlock(
    #[prop(into)] label: String,
    #[prop(optional, into)] hint: Option<String>,
    children: Children,
) -> impl IntoView {
    let hint_view = hint.map(|text| view! { <p class="field-hint">{text}</p> }.into_any());

    view! {
        <label class="field-block">
            <span class="field-label">{label}</span>
            {hint_view}
            <div class="field-control">{children()}</div>
        </label>
    }
}
