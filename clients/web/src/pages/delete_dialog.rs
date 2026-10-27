use leptos::prelude::*;

use protocol::Node;

#[component]
pub fn DeleteDialog(
    nodes: Vec<Node>,
    deleting: RwSignal<Option<Vec<Node>>>,
    on_confirm: Callback<Vec<Node>>,
) -> impl IntoView {
    let title = match nodes.as_slice() {
        [node] => format!("Move “{}” to recycle bin?", node.name),
        _ => format!("Move {} items to recycle bin?", nodes.len()),
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
                </header>
                <div class="modal__actions">
                    <button class="button" on:click=move |_| deleting.set(None)>
                        "Cancel"
                    </button>
                    <button class="button button--primary" on:click=confirm>
                        "Move to bin"
                    </button>
                </div>
            </div>
        </div>
    }
}
