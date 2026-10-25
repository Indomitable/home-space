use leptos::prelude::*;
use leptos_router::components::{Outlet, Redirect};

use super::app_bar::AppBar;
use crate::session::Auth;

#[component]
pub fn Shell(auth: Auth) -> impl IntoView {
    view! {
        <Show
            when=move || auth.user.get().is_some()
            fallback=|| view! { <Redirect path="/login" /> }
        >
            <div class="browser">
                <AppBar auth />
                <Outlet />
            </div>
        </Show>
    }
}
