use leptos::prelude::*;

use api_client::format::human_size;
use protocol::{Conflict, Node, OnConflict};

use crate::locale::short_date;

/// Lists files that clash at a destination and asks what to do with them.
///
/// `on_choose` is called with the chosen mode, or `None` when the dialog is cancelled.
#[component]
pub fn ConflictDialog(
    title: String,
    conflicts: Vec<Conflict>,
    on_choose: Callback<Option<OnConflict>>,
) -> impl IntoView {
    let rows = conflicts
        .into_iter()
        .map(|conflict| {
            let existing = conflict.existing.as_ref().map(describe);
            view! {
                <li class="conflict-list__item">
                    <p class="conflict-list__name">{conflict.source.name.clone()}</p>
                    <p class="conflict-list__path">{conflict.path.clone()}</p>
                    <dl class="conflict-list__versions">
                        <dt>"Incoming"</dt>
                        <dd>{describe(&conflict.source)}</dd>
                        <dt>"Existing"</dt>
                        <dd>{existing.unwrap_or_else(|| "Also selected".to_owned())}</dd>
                    </dl>
                </li>
            }
        })
        .collect_view();

    view! {
        <div class="modal" on:click=move |_| on_choose.run(None)>
            <div
                class="modal__panel modal__panel--wide"
                role="alertdialog"
                aria-modal="true"
                on:click=|event| event.stop_propagation()
            >
                <header class="modal__header">
                    <h2 class="modal__title">{title}</h2>
                    <p class="modal__subtitle">
                        "Override replaces the current content and keeps it as a version."
                    </p>
                </header>
                <ul class="conflict-list">{rows}</ul>
                <div class="modal__actions">
                    <button class="button" on:click=move |_| on_choose.run(None)>
                        "Cancel"
                    </button>
                    <button
                        class="button"
                        on:click=move |_| on_choose.run(Some(OnConflict::Skip))
                    >
                        "Skip these files"
                    </button>
                    <button
                        class="button button--danger"
                        on:click=move |_| on_choose.run(Some(OnConflict::Override))
                    >
                        "Override"
                    </button>
                </div>
            </div>
        </div>
    }
}

fn describe(node: &Node) -> String {
    format!(
        "{} · {}",
        short_date(node.updated_at),
        human_size(node.size)
    )
}
