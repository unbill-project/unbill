mod api;
mod app;
mod components;
mod pages;

build_info::build_info!(fn compiled_build_info);

use std::sync::Arc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use unbill_asymmetric_channel::AsymChannel;
use unbill_asymmetric_channel::http::HttpAsymChannel;
use unbill_console::service::UnbillConsole;

use components::{ActionButton, FieldBlock, ToastProvider, apply_theme, load_theme};

const SESSION_STORAGE_KEY: &str = "unbill_account_session";

// sirno:witness:unbill-ui-remote:begin
fn server_base_url() -> String {
    match option_env!("UNBILL_SERVER_URL") {
        Some(url) if !url.is_empty() => url.to_owned(),
        _ => web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default(),
    }
}

fn read_stored_session() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(SESSION_STORAGE_KEY).ok().flatten())
        .filter(|k| !k.is_empty())
}

fn write_stored_session(token: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(SESSION_STORAGE_KEY, token);
    }
}

fn clear_stored_session() {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.remove_item(SESSION_STORAGE_KEY);
    }
}

async fn init_service(session_token: String) -> Result<String, String> {
    let base_url = server_base_url();
    let channel = HttpAsymChannel::open(base_url, session_token)
        .await
        .map_err(|e| e.to_string())?;
    let name = channel.current_account().await.map_err(|e| e.to_string())?;
    let service = UnbillConsole::open(Arc::new(channel) as Arc<dyn AsymChannel>).await;
    api::init(service);
    Ok(name)
}
// sirno:witness:unbill-ui-remote:end

#[component]
fn Root() -> impl IntoView {
    let ready = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let name_input = RwSignal::new(String::new());
    let password_input = RwSignal::new(String::new());
    let account_name = RwSignal::new(String::new());
    let account_menu = RwSignal::new(false);
    let old_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());

    // Attempt silent login with the stored session on startup.
    if let Some(token) = read_stored_session() {
        spawn_local(async move {
            match init_service(token).await {
                Ok(name) => {
                    account_name.set(name);
                    ready.set(true);
                }
                Err(_) => clear_stored_session(),
            }
        });
    }

    let connect = move |register: bool| {
        let name = name_input.get_untracked().trim().to_owned();
        let password = password_input.get_untracked();
        if name.is_empty() || password.is_empty() {
            error.set(Some("Enter an account name and password.".to_owned()));
            return;
        }
        spawn_local(async move {
            let base = server_base_url();
            if register && let Err(e) = HttpAsymChannel::register_at(&base, &name, &password).await
            {
                error.set(Some(e.to_string()));
                return;
            }
            match HttpAsymChannel::login_at(&base, &name, &password).await {
                Ok(token) => match init_service(token.clone()).await {
                    Ok(name) => {
                        write_stored_session(&token);
                        account_name.set(name);
                        password_input.set(String::new());
                        ready.set(true);
                    }
                    Err(e) => error.set(Some(e)),
                },
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    let logout = move |_| {
        spawn_local(async move {
            if let Err(e) = api::logout().await {
                error.set(Some(e));
            }
            clear_stored_session();
            account_menu.set(false);
            ready.set(false);
        });
    };

    let change_password = move |_| {
        let old = old_password.get_untracked();
        let new = new_password.get_untracked();
        spawn_local(async move {
            match api::change_password(&old, &new).await {
                Ok(()) => {
                    clear_stored_session();
                    account_menu.set(false);
                    ready.set(false);
                    old_password.set(String::new());
                    new_password.set(String::new());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        {move || {
            if ready.get() {
                view! {
                    <app::App />
                    <div class="account-toolbar">
                        <button type="button" class="account-trigger" aria-expanded=move || account_menu.get().to_string() on:click=move |_| account_menu.update(|open| *open = !*open)>{move || account_name.get()}</button>
                        {move || account_menu.get().then(|| view! {
                            <div class="account-menu">
                                <label>"Current password"<input class="ui-input" type="password" prop:value=move || old_password.get() on:input=move |ev| old_password.set(event_target_value(&ev)) /></label>
                                <label>"New password"<input class="ui-input" type="password" prop:value=move || new_password.get() on:input=move |ev| new_password.set(event_target_value(&ev)) /></label>
                                <button type="button" class="action-button action-button-primary action-button-block" on:click=change_password>"Change password"</button>
                                <button type="button" class="action-button action-button-quiet action-button-block" on:click=logout>"Log out"</button>
                            </div>
                        })}
                    </div>
                }.into_any()
            } else {
                view! {
                    <main class="login-shell">
                        <div class="login-form">
                            <h1 class="login-title">"unbill"</h1>
                            <FieldBlock label="Account name".to_owned()>
                                <input class="ui-input" type="text" prop:value=move || name_input.get() on:input=move |ev| name_input.set(event_target_value(&ev)) />
                            </FieldBlock>
                            <FieldBlock label="Password".to_owned()>
                                <input
                                    class="ui-input"
                                    type="password"
                                    prop:value=move || password_input.get()
                                    on:input=move |ev| password_input.set(event_target_value(&ev))
                                />
                            </FieldBlock>
                            {move || error.get().map(|e| view! {
                                <p class="form-error">{e}</p>
                            })}
                            <ActionButton
                                label="Log in".to_owned()
                                full_width=true
                                on_press=Callback::new(move |_| connect(false))
                            />
                            <ActionButton label="Register".to_owned() full_width=true on_press=Callback::new(move |_| connect(true)) />
                            <unbill_ui_components::about::OpenAbout />
                            <unbill_ui_components::licenses::OpenSourceLicenses />
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
    apply_theme(load_theme());
    mount_to_body(|| view! { <ToastProvider><Root /></ToastProvider> });
}
