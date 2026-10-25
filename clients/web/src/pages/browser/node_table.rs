use leptos::{html::Input, prelude::*};
use leptos_router::components::A;
use web_sys::KeyboardEvent;

use protocol::{Node, NodeId, NodeKind};

use crate::{
    format::{human_size, short_date},
    route,
};

const KEY_ENTER: &str = "Enter";
const KEY_ESCAPE: &str = "Escape";

#[derive(Clone, Copy)]
pub struct TableActions {
    pub draft: RwSignal<Option<String>>,
    pub renaming: RwSignal<Option<(NodeId, String)>>,
    pub saving: RwSignal<bool>,
    pub moving: RwSignal<Option<Node>>,
    pub create_folder: Callback<()>,
    pub cancel_folder: Callback<()>,
    pub save_rename: Callback<()>,
    pub cancel_rename: Callback<()>,
    pub download: Callback<Node>,
    pub delete: Callback<Node>,
}

#[component]
pub fn NodeTable(
    nodes: RwSignal<Vec<Node>>,
    actions: TableActions,
    draft_input: NodeRef<Input>,
) -> impl IntoView {
    view! {
        <table class="node-table">
            <thead class="node-table__head">
                <tr>
                    <th class="node-table__header-cell">"Name"</th>
                    <th class="node-table__header-cell node-table__header-cell--size">"Size"</th>
                    <th class="node-table__header-cell node-table__header-cell--modified">
                        "Modified"
                    </th>
                    <th class="node-table__header-cell">
                        <span class="visually-hidden">"Actions"</span>
                    </th>
                </tr>
            </thead>
            <tbody class="node-table__body">
                <Show when=move || actions.draft.read().is_some()>
                    <DraftRow actions draft_input />
                </Show>
                <For
                    each=move || nodes.get()
                    key=|node| (node.id, node.updated_at.unix_timestamp_nanos())
                    children=move |node| view! { <NodeRow node actions /> }
                />
            </tbody>
        </table>
    }
}

#[component]
fn DraftRow(actions: TableActions, draft_input: NodeRef<Input>) -> impl IntoView {
    let TableActions {
        draft,
        saving,
        create_folder,
        cancel_folder,
        ..
    } = actions;

    view! {
        <tr class="node-table__row node-table__row--draft">
            <td class="node-table__cell">
                <span class="node-table__name">
                    <span class="node-table__icon" aria-hidden="true">
                        "📁"
                    </span>
                    <input
                        node_ref=draft_input
                        class="field__input node-table__input"
                        type="text"
                        placeholder="Folder name"
                        aria-label="Folder name"
                        prop:value=move || draft.get().unwrap_or_default()
                        disabled=move || saving.get()
                        on:input:target=move |event| draft.set(Some(event.target().value()))
                        on:keydown=move |event: KeyboardEvent| {
                            match event.key().as_str() {
                                KEY_ENTER => create_folder.run(()),
                                KEY_ESCAPE => cancel_folder.run(()),
                                _ => {}
                            }
                        }
                    />
                </span>
            </td>
            <td class="node-table__cell node-table__cell--size"></td>
            <td class="node-table__cell node-table__cell--modified"></td>
            <td class="node-table__cell node-table__cell--actions">
                <div class="node-table__actions">
                    <button
                        class="button button--primary button--small"
                        disabled=move || saving.get()
                        on:click=move |_| create_folder.run(())
                    >
                        "Create"
                    </button>
                    <button
                        class="button button--ghost button--small"
                        disabled=move || saving.get()
                        on:click=move |_| cancel_folder.run(())
                    >
                        "Cancel"
                    </button>
                </div>
            </td>
        </tr>
    }
}

#[component]
fn NodeRow(node: Node, actions: TableActions) -> impl IntoView {
    let TableActions {
        renaming,
        saving,
        moving,
        save_rename,
        cancel_rename,
        download,
        delete,
        ..
    } = actions;
    let is_folder = node.kind == NodeKind::Folder;
    let name = node.name.clone();
    let size = if is_folder {
        String::new()
    } else {
        human_size(node.size)
    };
    let modified = short_date(node.updated_at);
    let href = route::folder(node.id);
    let node_id = node.id;
    let editing = move || {
        renaming
            .read()
            .as_ref()
            .is_some_and(|(id, _)| *id == node_id)
    };
    let rename_input = NodeRef::<Input>::new();
    Effect::new(move |_| {
        if let Some(input) = rename_input.get() {
            let _ = input.focus();
            input.select();
        }
    });

    view! {
        <tr class="node-table__row">
            <td class="node-table__cell">
                {move || if editing() {
                    view! {
                        <span class="node-table__name node-table__name--editing">
                            <span class="node-table__icon" aria-hidden="true">
                                {if is_folder { "📁" } else { "📄" }}
                            </span>
                            <input
                                node_ref=rename_input
                                class="field__input node-table__input"
                                type="text"
                                aria-label="New name"
                                prop:value=move || {
                                    renaming
                                        .read()
                                        .as_ref()
                                        .map(|(_, name)| name.clone())
                                        .unwrap_or_default()
                                }
                                disabled=move || saving.get()
                                on:input:target=move |event| {
                                    renaming.set(Some((node_id, event.target().value())))
                                }
                                on:keydown=move |event: KeyboardEvent| {
                                    match event.key().as_str() {
                                        KEY_ENTER => save_rename.run(()),
                                        KEY_ESCAPE => cancel_rename.run(()),
                                        _ => {}
                                    }
                                }
                            />
                        </span>
                    }
                        .into_any()
                } else if is_folder {
                    let (href, name) = (href.clone(), name.clone());
                    view! {
                        <A href=href attr:class="node-table__name node-table__name--link">
                            <span class="node-table__icon" aria-hidden="true">
                                "📁"
                            </span>
                            {name}
                        </A>
                    }
                        .into_any()
                } else {
                    view! {
                        <span class="node-table__name">
                            <span class="node-table__icon" aria-hidden="true">
                                "📄"
                            </span>
                            {name.clone()}
                        </span>
                    }
                        .into_any()
                }}
            </td>
            <td class="node-table__cell node-table__cell--size">{size}</td>
            <td class="node-table__cell node-table__cell--modified">{modified}</td>
            <td class="node-table__cell node-table__cell--actions">
                {move || if editing() {
                    view! {
                        <div class="node-table__actions node-table__actions--editing">
                            <button
                                class="button button--primary button--small"
                                disabled=move || saving.get()
                                on:click=move |_| save_rename.run(())
                            >
                                "Save"
                            </button>
                            <button
                                class="button button--ghost button--small"
                                disabled=move || saving.get()
                                on:click=move |_| cancel_rename.run(())
                            >
                                "Cancel"
                            </button>
                        </div>
                    }
                        .into_any()
                } else {
                    let (for_download, for_rename, for_move, for_delete) = (
                        node.clone(),
                        node.clone(),
                        node.clone(),
                        node.clone(),
                    );
                    view! {
                        <div class="node-table__actions">
                            {(!is_folder)
                                .then(|| {
                                    view! {
                                        <button
                                            class="button button--ghost button--small"
                                            on:click=move |_| download.run(for_download.clone())
                                        >
                                            "Download"
                                        </button>
                                    }
                                })}
                            <button
                                class="button button--ghost button--small"
                                on:click=move |_| {
                                    renaming.set(Some((for_rename.id, for_rename.name.clone())))
                                }
                            >
                                "Rename"
                            </button>
                            <button
                                class="button button--ghost button--small"
                                on:click=move |_| moving.set(Some(for_move.clone()))
                            >
                                "Move"
                            </button>
                            <button
                                class="button button--ghost button--small button--danger"
                                on:click=move |_| delete.run(for_delete.clone())
                            >
                                "Delete"
                            </button>
                        </div>
                    }
                        .into_any()
                }}
            </td>
        </tr>
    }
}
