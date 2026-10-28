use leptos::prelude::*;

/// An error banner, shown while `message` is set.
#[component]
pub fn ErrorAlert(#[prop(into)] message: Signal<Option<String>>) -> impl IntoView {
    move || {
        message
            .get()
            .map(|message| view! { <p class="alert alert--error" role="alert">{message}</p> })
    }
}
