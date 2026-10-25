use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use leptos_router::components::Redirect;

use crate::session::Auth;

#[component]
pub fn Login(auth: Auth) -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        busy.set(true);
        spawn_local(async move {
            let result = auth
                .login(&username.get_untracked(), &password.get_untracked())
                .await;
            busy.set(false);
            match result {
                Ok(()) => error.set(None),
                Err(message) => error.set(Some(message)),
            }
            password.set(String::new());
        });
    };

    view! {
        <Show when=move || auth.user.get().is_some()>
            <Redirect path="/" />
        </Show>
        <main class="login">
            <h1>"home-space"</h1>
            <form on:submit=submit>
                <label>
                    "Username"
                    <input
                        type="text"
                        autocomplete="username"
                        required
                        prop:value=move || username.get()
                        on:input:target=move |event| username.set(event.target().value())
                    />
                </label>
                <label>
                    "Password"
                    <input
                        type="password"
                        autocomplete="current-password"
                        required
                        prop:value=move || password.get()
                        on:input:target=move |event| password.set(event.target().value())
                    />
                </label>
                {move || {
                    error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })
                }}
                <button type="submit" class="primary" disabled=move || busy.get()>
                    "Log in"
                </button>
            </form>
        </main>
    }
}
