mod api;
mod app;
mod components;
mod pages;

use std::sync::Arc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use unbill_core::service::UnbillService;
use unbill_store_http::{HttpStore, LockedHttpStore};

use components::{ActionButton, FieldBlock, ToastProvider};

const API_KEY_STORAGE_KEY: &str = "unbill_api_key";

fn server_base_url() -> String {
    match option_env!("UNBILL_SERVER_URL") {
        Some(url) if !url.is_empty() => url.to_owned(),
        _ => web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default(),
    }
}

fn read_stored_key() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(API_KEY_STORAGE_KEY).ok().flatten())
        .filter(|k| !k.is_empty())
}

fn write_stored_key(key: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(API_KEY_STORAGE_KEY, key);
    }
}

async fn init_service(api_key: String) -> Result<(), String> {
    let base_url = server_base_url();
    let store = Arc::new(LockedHttpStore::new(HttpStore::new(
        base_url.clone(),
        api_key.clone(),
    )));
    let service = UnbillService::open_remote(store, base_url, api_key)
        .await
        .map_err(|e| e.to_string())?;
    api::init(service);
    Ok(())
}

#[component]
fn Root() -> impl IntoView {
    let ready = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let key_input = RwSignal::new(String::new());

    // Attempt silent login with stored key on startup.
    if let Some(key) = read_stored_key() {
        spawn_local(async move {
            if init_service(key).await.is_ok() {
                ready.set(true);
            }
        });
    }

    let connect = move |_| {
        let key = key_input.get_untracked().trim().to_owned();
        if key.is_empty() {
            error.set(Some("Enter an API key.".to_owned()));
            return;
        }
        spawn_local(async move {
            match init_service(key.clone()).await {
                Ok(()) => {
                    write_stored_key(&key);
                    ready.set(true);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        {move || {
            if ready.get() {
                view! { <app::App /> }.into_any()
            } else {
                view! {
                    <main class="login-shell">
                        <div class="login-form">
                            <h1 class="login-title">"unbill"</h1>
                            <FieldBlock label="API Key".to_owned()>
                                <input
                                    class="ui-input"
                                    type="password"
                                    placeholder="Paste your API key"
                                    prop:value=move || key_input.get()
                                    on:input=move |ev| key_input.set(event_target_value(&ev))
                                />
                            </FieldBlock>
                            {move || error.get().map(|e| view! {
                                <p class="form-error">{e}</p>
                            })}
                            <ActionButton
                                label="Connect".to_owned()
                                full_width=true
                                on_press=Callback::new(connect)
                            />
                        </div>
                    </main>
                }
                .into_any()
            }
        }}
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <ToastProvider><Root /></ToastProvider> });
}
