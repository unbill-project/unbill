use leptos::prelude::*;

// sirno:witness:ui-components:begin
#[component]
pub fn BluetoothBadge() -> impl IntoView {
    view! {
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" role="img" aria-label="BLE connected"
            style="color: #3b82f6; vertical-align: middle; margin-inline-start: 0.35em;">
            <title>"BLE connected"</title>
            <path d="m7 7 10 10-5 5V2l5 5L7 17" />
        </svg>
    }
}
// sirno:witness:ui-components:end
