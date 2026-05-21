use leptos::prelude::*;

const STORAGE_KEY: &str = "unbill-theme";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}

/// Read the persisted theme preference from localStorage.
pub fn load_theme() -> ThemeMode {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
        .map(|v| ThemeMode::from_str(&v))
        .unwrap_or(ThemeMode::System)
}

/// Apply a theme mode by setting `data-theme` on `<html>` and persisting to localStorage.
pub fn apply_theme(mode: ThemeMode) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(root) = document.document_element() else {
        return;
    };

    match mode {
        ThemeMode::System => {
            let _ = root.remove_attribute("data-theme");
        }
        ThemeMode::Light | ThemeMode::Dark => {
            let _ = root.set_attribute("data-theme", mode.as_str());
        }
    }

    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.set_item(STORAGE_KEY, mode.as_str());
    }
}

/// A three-way theme picker: System / Light / Dark.
#[component]
pub fn ThemePicker(
    #[prop(into)] mode: Signal<ThemeMode>,
    on_change: Callback<ThemeMode>,
) -> impl IntoView {
    let pill = move |label: &'static str, value: ThemeMode| {
        let class_name = move || {
            if mode.get() == value {
                "tag-pill tag-pill-active"
            } else {
                "tag-pill"
            }
        };
        view! {
            <button type="button" class=class_name on:click=move |_| on_change.run(value)>
                {label}
            </button>
        }
    };

    view! {
        <div class="chip-row">
            {pill("Light", ThemeMode::Light)}
            {pill("System", ThemeMode::System)}
            {pill("Dark", ThemeMode::Dark)}
        </div>
    }
}
