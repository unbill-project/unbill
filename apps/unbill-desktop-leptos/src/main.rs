use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <main class="shell">
            <section class="panel">
                <p class="eyebrow">"Leptos Frontend"</p>
                <h1 class="title">"Unbill"</h1>
                <p class="copy">
                    "This Tauri shell is now wired to an alternate Leptos + Trunk frontend."
                    " Keep the existing React app for comparison, or move feature work here."
                </p>
                <div class="status">"Ready for Tauri API wiring"</div>
            </section>
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}
