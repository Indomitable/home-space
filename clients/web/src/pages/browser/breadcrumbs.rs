use leptos::prelude::*;
use leptos_router::components::A;

use protocol::Node;

use crate::route;

#[component]
pub fn Breadcrumbs(crumbs: RwSignal<Vec<Node>>) -> impl IntoView {
    view! {
        <nav class="breadcrumbs" aria-label="Folder path">
            <A href="/" exact=true attr:class="breadcrumbs__link">
                "Home"
            </A>
            <For
                each=move || crumbs.get()
                key=|folder| folder.id
                children=move |folder| {
                    view! {
                        <span class="breadcrumbs__separator" aria-hidden="true">
                            "/"
                        </span>
                        <A
                            href=route::folder(folder.id)
                            exact=true
                            attr:class="breadcrumbs__link"
                        >
                            {folder.name.clone()}
                        </A>
                    }
                }
            />
        </nav>
    }
}
