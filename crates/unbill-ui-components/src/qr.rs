use leptos::prelude::*;

/// Render a QR code as an inline SVG for the given data string.
#[component]
pub fn QrCode(#[prop(into)] data: String) -> impl IntoView {
    let svg = qrcode::QrCode::new(data.as_bytes())
        .map(|code| {
            use qrcode::render::svg;
            code.render::<svg::Color>().quiet_zone(true).build()
        })
        .unwrap_or_default();

    view! { <div class="invite-qr" inner_html=svg></div> }
}
