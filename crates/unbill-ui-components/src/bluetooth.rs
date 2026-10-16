use leptos::prelude::*;

// sirno:witness:ui-components:begin
#[component]
pub fn BluetoothBadge(connected: bool) -> impl IntoView {
    let label = if connected {
        "Connected via Bluetooth"
    } else {
        "Device seen, not connected"
    };
    let color = if connected {
        "#3b82f6"
    } else {
        "var(--text-soft)"
    };
    view! {
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" role="img" aria-label=label
            style=format!("color: {color}; vertical-align: middle; margin-inline-start: 0.35em;")>
            <title>{label}</title>
            <path d="m7 7 10 10-5 5V2l5 5L7 17" />
        </svg>
    }
}
// sirno:witness:ui-components:end

#[component]
pub fn BluetoothLegend() -> impl IntoView {
    view! {
        <p class="row-meta"><BluetoothBadge connected=true />" Blue: connected via Bluetooth."</p>
        <p class="row-meta"><BluetoothBadge connected=false />" Gray: device seen, not connected."</p>
    }
}
