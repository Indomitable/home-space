use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use leptos_router::components::Redirect;

use crate::{components::ErrorAlert, session::Auth};

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
            <section class="login__card">
                <header class="login__header">
                    <h1 class="login__title">"home-space"</h1>
                    <p class="login__subtitle">"Sign in to your files"</p>
                </header>
                <form class="login__form" on:submit=submit>
                    <label class="field">
                        <span class="field__label">"Username"</span>
                        <input
                            class="field__input"
                            type="text"
                            autocomplete="username"
                            required
                            prop:value=move || username.get()
                            on:input:target=move |event| username.set(event.target().value())
                        />
                    </label>
                    <label class="field">
                        <span class="field__label">"Password"</span>
                        <input
                            class="field__input"
                            type="password"
                            autocomplete="current-password"
                            required
                            prop:value=move || password.get()
                            on:input:target=move |event| password.set(event.target().value())
                        />
                    </label>
                    <ErrorAlert message=error />
                    <button
                        type="submit"
                        class="button button--primary button--block"
                        disabled=move || busy.get()
                    >
                        "Log in"
                    </button>
                </form>
            </section>
        </main>
    }
}
