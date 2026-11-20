mod api;
mod app;
mod components;
mod pages;

build_info::build_info!(fn compiled_build_info);

use crate::components::ToastProvider;
use crate::components::{apply_theme, load_theme};
use leptos::prelude::*;
use leptos::task::spawn_local;

const SESSION_KEY: &str = "unbill_account_session";

fn stored_token() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(SESSION_KEY).ok().flatten())
}

fn save_token(token: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(SESSION_KEY, token);
    }
}

fn clear_token() {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.remove_item(SESSION_KEY);
    }
}

#[component]
fn Root() -> impl IntoView {
    let ready = RwSignal::new(false);
    let loading = RwSignal::new(true);
    let error = RwSignal::new(None::<String>);
    let account_name = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let old_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let account_menu = RwSignal::new(false);

    spawn_local(async move {
        let account = match api::account_status().await {
            Ok(name) => Ok(name),
            Err(_) => match stored_token() {
                Some(token) => api::resume_account(&token).await,
                None => Err("no session".to_owned()),
            },
        };
        if let Ok(name) = account {
            account_name.set(name);
            ready.set(true);
        } else {
            clear_token();
        }
        loading.set(false);
    });

    let authenticate = move |register: bool| {
        let account = name.get_untracked().trim().to_owned();
        let secret = password.get_untracked();
        if account.is_empty() || secret.is_empty() {
            error.set(Some("Enter an account name and password.".to_owned()));
            return;
        }
        spawn_local(async move {
            if register {
                if let Err(e) = api::register_account(&account, &secret).await {
                    error.set(Some(e));
                    return;
                }
            }
            match api::login_account(&account, &secret).await {
                Ok(token) => {
                    save_token(&token);
                    account_name.set(account);
                    password.set(String::new());
                    ready.set(true);
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let logout = move |_| {
        spawn_local(async move {
            if let Err(e) = api::logout_account().await {
                error.set(Some(e));
            }
            clear_token();
            ready.set(false);
            account_menu.set(false);
        });
    };
    let change_password = move |_| {
        let old = old_password.get_untracked();
        let new = new_password.get_untracked();
        spawn_local(async move {
            match api::change_account_password(&old, &new).await {
                Ok(()) => {
                    clear_token();
                    ready.set(false);
                    account_menu.set(false);
                    old_password.set(String::new());
                    new_password.set(String::new());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        {move || {
            if loading.get() {
                view! { <main class="login-shell"><p>"Connecting…"</p></main> }.into_any()
            } else if ready.get() {
                view! {
                    <app::App />
                    <div class="account-toolbar">
                        <button type="button" on:click=move |_| account_menu.update(|open| *open = !*open)>{move || account_name.get()}</button>
                        {move || account_menu.get().then(|| view! {
                            <div class="account-menu">
                                <label>"Current password"<input type="password" prop:value=move || old_password.get() on:input=move |ev| old_password.set(event_target_value(&ev)) /></label>
                                <label>"New password"<input type="password" prop:value=move || new_password.get() on:input=move |ev| new_password.set(event_target_value(&ev)) /></label>
                                <button type="button" on:click=change_password>"Change password"</button>
                                <button type="button" on:click=logout>"Log out"</button>
                            </div>
                        })}
                    </div>
                }.into_any()
            } else {
                view! {
                    <main class="login-shell"><div class="login-form">
                        <h1 class="login-title">"unbill"</h1>
                        <label>"Account name"<input class="ui-input" type="text" prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev)) /></label>
                        <label>"Password"<input class="ui-input" type="password" prop:value=move || password.get() on:input=move |ev| password.set(event_target_value(&ev)) /></label>
                        {move || error.get().map(|message| view! { <p class="form-error">{message}</p> })}
                        <button type="button" on:click=move |_| authenticate(false)>"Log in"</button>
                        <button type="button" on:click=move |_| authenticate(true)>"Register"</button>
                    </div></main>
                }.into_any()
            }
        }}
    }
}

fn main() {
    console_error_panic_hook::set_once();
    apply_theme(load_theme());
    mount_to_body(|| view! { <ToastProvider><Root /></ToastProvider> });
}
