use leptos::prelude::*;

#[component]
pub fn CircularProgress() -> impl IntoView {
    view! {
        <svg class="circular-progress" viewBox="0 0 32 32" aria-hidden="true">
            <circle class="circular-progress-track" cx="16" cy="16" r="12" />
            <circle class="circular-progress-fill" cx="16" cy="16" r="12" />
        </svg>
    }
}
