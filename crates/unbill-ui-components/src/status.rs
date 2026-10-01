use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen_futures::JsFuture;

const TOAST_DURATION_MS: i32 = 8300;

#[derive(Clone, PartialEq)]
struct ToastEntry {
    id: u64,
    message: String,
    is_error: bool,
}

/// Context handle returned by [`use_toast`]. Copy — capture it freely in closures.
#[derive(Clone, Copy)]
pub struct ToastContext {
    entries: RwSignal<Vec<ToastEntry>>,
    next_id: RwSignal<u64>,
}

impl ToastContext {
    /// Push a success toast.
    pub fn show(self, message: String) {
        self.push(message, false);
    }

    /// Push an error toast.
    pub fn error(self, message: String) {
        self.push(message, true);
    }

    fn push(self, message: String, is_error: bool) {
        let id = self.next_id.get_untracked();
        #[allow(
            clippy::arithmetic_side_effects,
            reason = "An app session cannot realistically create u64::MAX toasts"
        )]
        self.next_id.update(|n| *n += 1);
        self.entries.update(|t| {
            t.push(ToastEntry {
                id,
                message,
                is_error,
            })
        });
    }
}

/// Access the nearest [`ToastProvider`] context. Panics if none is in the tree.
#[allow(
    clippy::expect_used,
    reason = "ToastProvider is required by this API and installed by both app roots"
)]
pub fn use_toast() -> ToastContext {
    use_context::<ToastContext>().expect("ToastProvider missing from component tree")
}

/// Wrap your app root with this component to enable toasts.
/// Use [`use_toast`] anywhere inside to push messages.
#[component]
pub fn ToastProvider(children: Children) -> impl IntoView {
    let entries: RwSignal<Vec<ToastEntry>> = RwSignal::new(vec![]);
    let next_id: RwSignal<u64> = RwSignal::new(1);
    provide_context(ToastContext { entries, next_id });

    view! {
        {children()}
        <div class="toast-container">
            <For
                each=move || entries.get()
                key=|entry| entry.id
                children=move |entry| {
                    let id = entry.id;
                    view! {
                        <ToastItem
                            entry=entry
                            on_dismiss=Callback::new(move |_| {
                                entries.update(|t| t.retain(|e| e.id != id));
                            })
                        />
                    }
                }
            />
        </div>
    }
}

#[component]
fn ToastItem(entry: ToastEntry, on_dismiss: Callback<()>) -> impl IntoView {
    // sirno:witness:ui-components:begin
    spawn_local(async move {
        #[allow(
            clippy::unwrap_used,
            reason = "ToastItem is rendered only inside a browser window"
        )]
        let window = web_sys::window().unwrap();
        let mut timer_scheduled = false;
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            timer_scheduled = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, TOAST_DURATION_MS)
                .is_ok();
        });
        // Keep the message visible if automatic dismissal is unavailable.
        if timer_scheduled && JsFuture::from(promise).await.is_ok() {
            on_dismiss.run(());
        }
    });
    // sirno:witness:ui-components:end

    let class = if entry.is_error {
        "toast toast-error"
    } else {
        "toast toast-info"
    };
    view! { <div class=class>{entry.message}</div> }
}
