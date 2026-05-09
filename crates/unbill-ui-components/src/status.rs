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
    spawn_local(async move {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, TOAST_DURATION_MS)
                .unwrap();
        });
        JsFuture::from(promise).await.ok();
        on_dismiss.run(());
    });

    let class = if entry.is_error {
        "toast toast-error"
    } else {
        "toast toast-info"
    };
    view! { <div class=class>{entry.message}</div> }
}
