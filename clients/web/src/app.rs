use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::{
    pages::{Browser, Login},
    session::Auth,
    uploads::{UploadItem, UploadPanel},
};

#[component]
pub fn App() -> impl IntoView {
    let auth = Auth::start();
    let uploads = RwSignal::new(Vec::<UploadItem>::new());
    provide_context(uploads);
    view! {
        <Router>
            <Show
                when=move || !auth.checking.get()
                fallback=|| view! { <p class="status">"Loading…"</p> }
            >
                <Routes fallback=|| view! { <p class="status">"Page not found"</p> }>
                    <Route path=path!("/login") view=move || view! { <Login auth /> } />
                    <Route path=path!("/") view=move || view! { <Browser auth /> } />
                    <Route path=path!("/folder/:id") view=move || view! { <Browser auth /> } />
                </Routes>
            </Show>
            <Show when=move || auth.user.read().is_some()>
                <UploadPanel uploads />
            </Show>
        </Router>
    }
}
