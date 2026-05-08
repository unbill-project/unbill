mod api;
mod app;
mod components;
mod pages;

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <app::App /> });
}
