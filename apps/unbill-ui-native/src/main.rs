mod api;
mod app;
mod components;
mod pages;

use crate::components::ToastProvider;
use crate::components::{apply_theme, load_theme};
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    apply_theme(load_theme());
    mount_to_body(|| view! { <ToastProvider><app::App /></ToastProvider> });
}
