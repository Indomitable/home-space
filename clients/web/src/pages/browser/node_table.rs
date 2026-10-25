use std::collections::HashSet;

use leptos::{html::Input, prelude::*};
use leptos_router::components::A;
use web_sys::KeyboardEvent;

use protocol::{Node, NodeId, NodeKind};

use super::menu::{Menu, MenuItem};
use crate::{
    format::{human_size, short_date},
    icon::{Icon, node_icon},
    route,
};

const KEY_ENTER: &str = "Enter";
const KEY_ESCAPE: &str = "Escape";

const ICON_CHECKED: &str = "check_box";
const ICON_UNCHECKED: &str = "check_box_outline_blank";
const ICON_MIXED: &str = "indeterminate_check_box";
const ICON_FOLDER: &str = "folder";

#[derive(Clone, Copy)]
pub struct TableActions {
    pub draft: RwSignal<Option<String>>,
    pub renaming: RwSignal<Option<(NodeId, String)>>,
    pub saving: RwSignal<bool>,
    pub moving: RwSignal<Option<Vec<Node>>>,
    pub selected: RwSignal<HashSet<NodeId>>,
    pub create_folder: Callback<()>,
    pub cancel_folder: Callback<()>,
    pub save_rename: Callback<()>,
    pub cancel_rename: Callback<()>,
    pub download: Callback<Node>,
    pub delete: Callback<Vec<Node>>,
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
                    <th class="node-table__header-cell node-table__header-cell--select">
                        <SelectAll nodes selected=actions.selected />
                    </th>
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

#[derive(Clone, Copy, PartialEq)]
enum Selection {
    None,
    Some,
    All,
}

fn selection_of(nodes: &[Node], selected: &HashSet<NodeId>) -> Selection {
    let count = nodes
        .iter()
        .filter(|node| selected.contains(&node.id))
        .count();
    if count == 0 {
        Selection::None
    } else if count == nodes.len() {
        Selection::All
    } else {
        Selection::Some
    }
}

#[component]
fn Checkbox(
    #[prop(into)] icon: Signal<&'static str>,
    #[prop(into)] checked: Signal<String>,
    #[prop(into)] label: Signal<String>,
    on_toggle: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            class="node-table__checkbox"
            role="checkbox"
            aria-checked=move || checked.get()
            aria-label=move || label.get()
            on:click=move |_| on_toggle.run(())
        >
            {move || view! { <Icon name=icon.get() /> }}
        </button>
    }
}

#[component]
fn SelectAll(nodes: RwSignal<Vec<Node>>, selected: RwSignal<HashSet<NodeId>>) -> impl IntoView {
    let state = Memo::new(move |_| selection_of(&nodes.read(), &selected.read()));
    let icon = move || match state.get() {
        Selection::None => ICON_UNCHECKED,
        Selection::Some => ICON_MIXED,
        Selection::All => ICON_CHECKED,
    };
    let checked = move || {
        match state.get() {
            Selection::None => "false",
            Selection::Some => "mixed",
            Selection::All => "true",
        }
        .to_owned()
    };
    let toggle = Callback::new(move |()| {
        if state.get_untracked() == Selection::All {
            selected.update(HashSet::clear);
        } else {
            selected.set(nodes.read_untracked().iter().map(|node| node.id).collect());
        }
    });
    view! { <Checkbox icon checked label="Select all" on_toggle=toggle /> }
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
            <td class="node-table__cell node-table__cell--select"></td>
            <td class="node-table__cell">
                <span class="node-table__name">
                    <Icon name=ICON_FOLDER />
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
        selected,
        save_rename,
        cancel_rename,
        download,
        delete,
        ..
    } = actions;
    let is_folder = node.kind == NodeKind::Folder;
    let icon = node_icon(&node);
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
    let is_selected = move || selected.read().contains(&node_id);
    let rename_input = NodeRef::<Input>::new();
    Effect::new(move |_| {
        if let Some(input) = rename_input.get() {
            let _ = input.focus();
            input.select();
        }
    });
    let toggle = Callback::new(move |()| {
        selected.update(|ids| {
            if !ids.remove(&node_id) {
                ids.insert(node_id);
            }
        });
    });
    let checkbox_icon = move || {
        if is_selected() {
            ICON_CHECKED
        } else {
            ICON_UNCHECKED
        }
    };
    let checkbox_state = move || is_selected().to_string();
    let checkbox_label = format!("Select {}", node.name);

    view! {
        <tr class="node-table__row" class:node-table__row--selected=is_selected>
            <td class="node-table__cell node-table__cell--select">
                <Checkbox
                    icon=Signal::derive(checkbox_icon)
                    checked=Signal::derive(checkbox_state)
                    label=checkbox_label
                    on_toggle=toggle
                />
            </td>
            <td class="node-table__cell">
                {move || if editing() {
                    view! {
                        <span class="node-table__name node-table__name--editing">
                            <Icon name=icon />
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
                            <Icon name=icon />
                            {name}
                        </A>
                    }
                        .into_any()
                } else {
                    view! {
                        <span class="node-table__name">
                            <Icon name=icon />
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
                        <div class="node-table__actions">
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
                    let target = StoredValue::new(node.clone());
                    view! {
                        <Menu
                            id=format!("node-menu-{}", route::short_id(node_id))
                            label=format!("Actions for {}", node.name)
                        >
                            {(!is_folder)
                                .then(|| {
                                    view! {
                                        <MenuItem
                                            icon="download"
                                            label="Download"
                                            on_select=Callback::new(move |()| {
                                                download.run(target.get_value())
                                            })
                                        />
                                    }
                                })}
                            <MenuItem
                                icon="edit"
                                label="Rename"
                                on_select=Callback::new(move |()| {
                                    let node = target.get_value();
                                    renaming.set(Some((node.id, node.name)))
                                })
                            />
                            <MenuItem
                                icon="drive_file_move"
                                label="Move"
                                on_select=Callback::new(move |()| {
                                    moving.set(Some(vec![target.get_value()]))
                                })
                            />
                            <MenuItem
                                icon="delete"
                                label="Delete"
                                danger=true
                                on_select=Callback::new(move |()| {
                                    delete.run(vec![target.get_value()])
                                })
                            />
                        </Menu>
                    }
                        .into_any()
                }}
            </td>
        </tr>
    }
}
