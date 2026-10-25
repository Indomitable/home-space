use leptos::prelude::*;

use protocol::Node;

#[component]
pub fn DeleteDialog(
    nodes: Vec<Node>,
    deleting: RwSignal<Option<Vec<Node>>>,
    on_confirm: Callback<Vec<Node>>,
) -> impl IntoView {
    let title = match nodes.as_slice() {
        [node] => format!("Delete “{}”?", node.name),
        _ => format!("Delete {} items?", nodes.len()),
    };
    let confirm = move |_| {
        if let Some(targets) = deleting.get_untracked() {
            on_confirm.run(targets);
        }
        deleting.set(None);
    };

    view! {
        <div class="modal" on:click=move |_| deleting.set(None)>
            <div
                class="modal__panel"
                role="alertdialog"
                aria-modal="true"
                on:click=|event| event.stop_propagation()
            >
                <header class="modal__header">
                    <h2 class="modal__title">{title}</h2>
                    <p class="modal__subtitle">"This cannot be undone."</p>
                </header>
                <div class="modal__actions">
                    <button class="button" on:click=move |_| deleting.set(None)>
                        "Cancel"
                    </button>
                    <button class="button button--danger" on:click=confirm>
                        "Delete"
                    </button>
                </div>
            </div>
        </div>
    }
}
