use leptos::prelude::*;

use crate::session::Auth;

#[component]
pub fn AppBar(auth: Auth) -> impl IntoView {
    view! {
        <header class="app-bar">
            <h1 class="app-bar__brand">"home-space"</h1>
            <span class="app-bar__spacer"></span>
            <span class="app-bar__user">{move || auth.user.get().map(|user| user.username)}</span>
            <button class="button button--ghost" on:click=move |_| auth.logout()>
                "Log out"
            </button>
        </header>
    }
}
