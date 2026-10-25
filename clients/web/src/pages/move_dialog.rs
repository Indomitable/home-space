use leptos::{prelude::*, task::spawn_local};

use protocol::{Node, NodeId, NodeKind};

use crate::{listing::sort_nodes, session, session::Auth};

#[component]
pub fn MoveDialog(
    node: Node,
    moving: RwSignal<Option<Node>>,
    reload: RwSignal<u32>,
    auth: Auth,
) -> impl IntoView {
    let node_id = node.id;
    let trail = RwSignal::new(Vec::<Node>::new());
    let folders = RwSignal::new(Vec::<Node>::new());
    let error = RwSignal::new(None::<String>);
    let target = Memo::new(move |_| trail.read().last().map(|folder| folder.id));

    Effect::new(move |_| {
        let wanted = target.get();
        spawn_local(async move {
            match session::client().list(wanted).await {
                Ok(listing) => {
                    let mut listing: Vec<Node> = listing
                        .into_iter()
                        .filter(|entry| entry.kind == NodeKind::Folder && entry.id != node_id)
                        .collect();
                    sort_nodes(&mut listing);
                    folders.set(listing);
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            }
        });
    });

    let confirm = move |_| {
        let destination: Option<NodeId> = target.get_untracked();
        spawn_local(async move {
            match session::client().move_to(node_id, destination).await {
                Ok(_) => {
                    moving.set(None);
                    reload.update(|count| *count += 1);
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            }
        });
    };

    view! {
        <div class="backdrop" on:click=move |_| moving.set(None)>
            <div class="dialog" role="dialog" on:click=|event| event.stop_propagation()>
                <h2>{format!("Move “{}”", node.name)}</h2>
                <p class="path">
                    {move || {
                        let names: Vec<String> = trail
                            .read()
                            .iter()
                            .map(|folder| folder.name.clone())
                            .collect();
                        format!("/{}", names.join("/"))
                    }}
                </p>
                <ul class="picker">
                    <Show when=move || !trail.read().is_empty()>
                        <li>
                            <button on:click=move |_| trail.update(|path| { path.pop(); })>
                                "↑ Up"
                            </button>
                        </li>
                    </Show>
                    <For
                        each=move || folders.get()
                        key=|folder| folder.id
                        children=move |folder| {
                            let name = folder.name.clone();
                            view! {
                                <li>
                                    <button on:click=move |_| {
                                        trail.update(|path| path.push(folder.clone()))
                                    }>{format!("📁 {name}")}</button>
                                </li>
                            }
                        }
                    />
                </ul>
                {move || {
                    error.get().map(|message| view! { <p class="error" role="alert">{message}</p> })
                }}
                <div class="dialog-actions">
                    <button on:click=move |_| moving.set(None)>"Cancel"</button>
                    <button class="primary" on:click=confirm>"Move here"</button>
                </div>
            </div>
        </div>
    }
}
