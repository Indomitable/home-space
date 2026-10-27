use std::collections::HashSet;

use leptos::{prelude::*, task::spawn_local};

use api_client::{
    format::conflicts_title,
    listing::{Sort, sort_nodes},
};
use protocol::{Conflict, Node, NodeId, NodeKind, OnConflict};

use super::conflict_dialog::ConflictDialog;
use crate::{icon::Icon, session, session::Auth};

#[component]
pub fn MoveDialog(
    nodes: Vec<Node>,
    moving: RwSignal<Option<Vec<Node>>>,
    selected: RwSignal<HashSet<NodeId>>,
    reload: RwSignal<u32>,
    auth: Auth,
) -> impl IntoView {
    let ids: Vec<NodeId> = nodes.iter().map(|node| node.id).collect();
    let title = match nodes.as_slice() {
        [node] => format!("Move “{}”", node.name),
        _ => format!("Move {} items", nodes.len()),
    };
    let moving_ids = ids.clone();
    let trail = RwSignal::new(Vec::<Node>::new());
    let folders = RwSignal::new(Vec::<Node>::new());
    let error = RwSignal::new(None::<String>);
    let target = Memo::new(move |_| trail.read().last().map(|folder| folder.id));

    Effect::new(move |_| {
        let wanted = target.get();
        let moving_ids = moving_ids.clone();
        spawn_local(async move {
            match session::client().list(wanted).await {
                Ok(listing) => {
                    let mut listing: Vec<Node> = listing
                        .into_iter()
                        .filter(|entry| {
                            entry.kind == NodeKind::Folder && !moving_ids.contains(&entry.id)
                        })
                        .collect();
                    sort_nodes(&mut listing, Sort::default());
                    folders.set(listing);
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            }
        });
    });

    // The title and clashes of a move that was held back, until the user chooses.
    let conflicts = RwSignal::new(None::<(String, Vec<Conflict>)>);
    let send = Callback::new(move |mode: OnConflict| {
        let destination: Option<NodeId> = target.get_untracked();
        let ids = ids.clone();
        spawn_local(async move {
            match session::client().move_nodes(&ids, destination, mode).await {
                Ok(response) if !response.conflicts.is_empty() => {
                    let name = response
                        .target
                        .map_or_else(|| "Home".to_owned(), |folder| folder.name);
                    let title = conflicts_title(response.conflicts.len(), &format!("“{name}”"));
                    conflicts.set(Some((title, response.conflicts)));
                }
                Ok(_) => {
                    moving.set(None);
                    selected.update(HashSet::clear);
                    reload.update(|count| *count += 1);
                }
                Err(err) => {
                    error.set(Some(auth.fail(&err)));
                    reload.update(|count| *count += 1);
                }
            }
        });
    });
    let choose = Callback::new(move |mode: Option<OnConflict>| {
        conflicts.set(None);
        if let Some(mode) = mode {
            send.run(mode);
        }
    });
    let confirm = move |_| send.run(OnConflict::Abort);

    view! {
        <div class="modal" on:click=move |_| moving.set(None)>
            <div
                class="modal__panel"
                role="dialog"
                aria-modal="true"
                on:click=|event| event.stop_propagation()
            >
                <header class="modal__header">
                    <h2 class="modal__title">{title}</h2>
                    <p class="modal__subtitle">
                        {move || {
                            let names: Vec<String> = trail
                                .read()
                                .iter()
                                .map(|folder| folder.name.clone())
                                .collect();
                            if names.is_empty() {
                                "To Home".to_string()
                            } else {
                                format!("To /{}", names.join("/"))
                            }
                        }}
                    </p>
                </header>
                <ul class="folder-picker">
                    <Show when=move || !trail.read().is_empty()>
                        <li>
                            <button
                                class="folder-picker__button folder-picker__button--up"
                                on:click=move |_| trail.update(|path| { path.pop(); })
                            >
                                <Icon name="arrow_upward" />
                                "Up"
                            </button>
                        </li>
                    </Show>
                    <Show when=move || folders.read().is_empty()>
                        <li class="folder-picker__empty">"No subfolders"</li>
                    </Show>
                    <For
                        each=move || folders.get()
                        key=|folder| folder.id
                        children=move |folder| {
                            let name = folder.name.clone();
                            view! {
                                <li>
                                    <button
                                        class="folder-picker__button"
                                        on:click=move |_| {
                                            trail.update(|path| path.push(folder.clone()))
                                        }
                                    >
                                        <Icon name="folder" />
                                        {name}
                                    </button>
                                </li>
                            }
                        }
                    />
                </ul>
                {move || {
                    error
                        .get()
                        .map(|message| {
                            view! { <p class="alert alert--error" role="alert">{message}</p> }
                        })
                }}
                <div class="modal__actions">
                    <button class="button" on:click=move |_| moving.set(None)>
                        "Cancel"
                    </button>
                    <button class="button button--primary" on:click=confirm>
                        "Move here"
                    </button>
                </div>
            </div>
        </div>
        {move || {
            conflicts
                .get()
                .map(|(title, conflicts)| {
                    view! { <ConflictDialog title conflicts on_choose=choose /> }
                })
        }}
    }
}
