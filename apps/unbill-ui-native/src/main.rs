mod api;
mod app;
mod components;
mod pages;

build_info::build_info!(fn compiled_build_info);

use crate::components::ToastProvider;
use crate::components::{apply_theme, load_theme};
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    apply_theme(load_theme());
    mount_to_body(|| view! { <ToastProvider><app::App /></ToastProvider> });
}
