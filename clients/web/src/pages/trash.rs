use std::collections::HashSet;

use leptos::{prelude::*, task::spawn_local};
use leptos_router::components::A;
use wasm_bindgen::JsCast;
use web_sys::{Element, MouseEvent};

use api_client::format::{conflicts_title, human_size};
use protocol::{Conflict, Event, NodeId, OnConflict, TrashItem};

use super::{
    browser::node_table::{
        Checkbox, ICON_CHECKED, ICON_MIXED, ICON_UNCHECKED, INTERACTIVE_SELECTOR,
    },
    conflict_dialog::ConflictDialog,
};
use crate::{
    events,
    icon::{Icon, node_icon},
    locale::short_date,
    route,
    session::{self, Auth},
};

const LOST_AND_FOUND: &str = "Lost & Found";

/// What a finished restore reports: how many files came back and where to.
#[derive(Clone)]
struct Restored {
    count: usize,
    folder: Option<NodeId>,
}

fn files(count: usize) -> String {
    match count {
        1 => "1 file".to_owned(),
        _ => format!("{count} files"),
    }
}

#[component]
pub fn TrashPage(auth: Auth) -> impl IntoView {
    let items = RwSignal::new(Vec::<TrashItem>::new());
    let selected = RwSignal::new(HashSet::<NodeId>::new());
    let error = RwSignal::new(None::<String>);
    let restored = RwSignal::new(None::<Restored>);
    let reload = RwSignal::new(0_u32);
    let conflicts = RwSignal::new(None::<(String, Vec<Conflict>)>);
    let emptying = RwSignal::new(None::<usize>);

    Effect::new(move |_| {
        reload.track();
        spawn_local(async move {
            match session::client().trash().await {
                Ok(listing) => {
                    items.set(listing);
                    error.set(None);
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            }
        });
    });
    Effect::new(move |_| {
        let listing = items.read();
        selected.update(|ids| ids.retain(|id| listing.iter().any(|item| item.node.id == *id)));
    });

    let hub = expect_context::<StoredValue<events::Hub, LocalStorage>>();
    let subscription = StoredValue::new_local(Some(hub.with_value(|hub| {
        hub.subscribe(move |event| {
            if matches!(event, Event::TrashChanged | Event::Resync) {
                reload.update(|count| *count += 1);
            }
        })
    })));
    on_cleanup(move || subscription.set_value(None));

    let restore = Callback::new(move |mode: OnConflict| {
        let ids: Vec<NodeId> = items
            .read_untracked()
            .iter()
            .map(|item| item.node.id)
            .filter(|id| selected.read_untracked().contains(id))
            .collect();
        if ids.is_empty() {
            return;
        }
        spawn_local(async move {
            match session::client().restore_trashed(&ids, mode).await {
                Ok(response) if !response.conflicts.is_empty() => {
                    let title = conflicts_title(response.conflicts.len(), LOST_AND_FOUND);
                    conflicts.set(Some((title, response.conflicts)));
                }
                Ok(response) => {
                    error.set(None);
                    restored.set(Some(Restored {
                        count: response.moved.len(),
                        folder: response.target.map(|folder| folder.id),
                    }));
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
            restore.run(mode);
        }
    });
    let confirm_empty = move |_| {
        emptying.set(None);
        spawn_local(async move {
            match session::client().empty_trash().await {
                Ok(()) => {
                    restored.set(None);
                    selected.update(HashSet::clear);
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            }
            reload.update(|count| *count += 1);
        });
    };

    let selected_count = move || selected.read().len();
    let is_empty = move || items.read().is_empty();

    view! {
        <main class="explorer">
            <div class="explorer__header">
                <nav class="breadcrumbs" aria-label="Folder path">
                    <A href="/" exact=true attr:class="breadcrumbs__link">
                        "Home"
                    </A>
                    <span class="breadcrumbs__separator" aria-hidden="true">
                        "/"
                    </span>
                    <span class="breadcrumbs__link" aria-current="page">
                        "Recycle bin"
                    </span>
                </nav>
                <div class="explorer__toolbar">
                    <button
                        class="button button--primary"
                        disabled=move || selected_count() == 0
                        on:click=move |_| {
                            restored.set(None);
                            restore.run(OnConflict::Abort)
                        }
                    >
                        <Icon name="restore_from_trash" />
                        "Restore"
                    </button>
                    <button
                        class="button button--danger"
                        disabled=is_empty
                        on:click=move |_| emptying.set(Some(items.read_untracked().len()))
                    >
                        <Icon name="delete_forever" />
                        "Empty recycle bin"
                    </button>
                </div>
            </div>
            {move || {
                error
                    .get()
                    .map(|message| {
                        view! { <p class="alert alert--error" role="alert">{message}</p> }
                    })
            }}
            {move || {
                restored
                    .get()
                    .map(|done| {
                        let message = format!("Restored {} to ", files(done.count));
                        let place = match done.folder {
                            Some(id) => {
                                view! { <A href=route::folder(id)>{LOST_AND_FOUND}</A> }.into_any()
                            }
                            None => LOST_AND_FOUND.into_any(),
                        };
                        view! {
                            <p class="alert alert--success" role="status">
                                {message} {place}
                            </p>
                        }
                    })
            }}
            <div class="explorer__scroll">
                <table class="node-table">
                    <thead class="node-table__head">
                        <tr>
                            <th class="node-table__header-cell node-table__header-cell--select">
                                <SelectAll items selected />
                            </th>
                            <th class="node-table__header-cell">"Name"</th>
                            <th class="node-table__header-cell node-table__header-cell--modified">
                                "Original path"
                            </th>
                            <th class="node-table__header-cell node-table__header-cell--modified">
                                "Deleted"
                            </th>
                            <th class="node-table__header-cell node-table__header-cell--size">
                                "Size"
                            </th>
                        </tr>
                    </thead>
                    <tbody class="node-table__body">
                        <For
                            each=move || items.get()
                            key=|item| item.node.id
                            children=move |item| view! { <TrashRow item selected /> }
                        />
                    </tbody>
                </table>
                <Show when=move || is_empty() && error.read().is_none()>
                    <p class="explorer__empty">"The recycle bin is empty."</p>
                </Show>
            </div>
        </main>
        {move || {
            conflicts
                .get()
                .map(|(title, conflicts)| {
                    view! { <ConflictDialog title conflicts on_choose=choose /> }
                })
        }}
        {move || {
            emptying
                .get()
                .map(|count| {
                    view! {
                        <div class="modal" on:click=move |_| emptying.set(None)>
                            <div
                                class="modal__panel"
                                role="alertdialog"
                                aria-modal="true"
                                on:click=|event| event.stop_propagation()
                            >
                                <header class="modal__header">
                                    <h2 class="modal__title">
                                        {format!("Permanently delete {}?", files(count))}
                                    </h2>
                                    <p class="modal__subtitle">"This cannot be undone."</p>
                                </header>
                                <div class="modal__actions">
                                    <button class="button" on:click=move |_| emptying.set(None)>
                                        "Cancel"
                                    </button>
                                    <button class="button button--danger" on:click=confirm_empty>
                                        "Empty recycle bin"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }
                })
        }}
    }
}

#[component]
fn SelectAll(
    items: RwSignal<Vec<TrashItem>>,
    selected: RwSignal<HashSet<NodeId>>,
) -> impl IntoView {
    let count = move || {
        let chosen = selected.read();
        items
            .read()
            .iter()
            .filter(|item| chosen.contains(&item.node.id))
            .count()
    };
    let all = move || !items.read().is_empty() && count() == items.read().len();
    let icon = move || match count() {
        0 => ICON_UNCHECKED,
        _ if all() => ICON_CHECKED,
        _ => ICON_MIXED,
    };
    let checked = move || {
        match count() {
            0 => "false",
            _ if all() => "true",
            _ => "mixed",
        }
        .to_owned()
    };
    let toggle = Callback::new(move |()| {
        if all() {
            selected.update(HashSet::clear);
        } else {
            selected.set(
                items
                    .read_untracked()
                    .iter()
                    .map(|item| item.node.id)
                    .collect(),
            );
        }
    });
    view! { <Checkbox icon checked label="Select all" on_toggle=toggle /> }
}

#[component]
fn TrashRow(item: TrashItem, selected: RwSignal<HashSet<NodeId>>) -> impl IntoView {
    let node_id = item.node.id;
    let icon = node_icon(&item.node);
    let is_selected = move || selected.read().contains(&node_id);
    let toggle = Callback::new(move |()| {
        selected.update(|ids| {
            if !ids.remove(&node_id) {
                ids.insert(node_id);
            }
        });
    });
    let select = move |event: MouseEvent| {
        let interactive = event
            .target()
            .and_then(|target| target.dyn_into::<Element>().ok())
            .is_some_and(|element| matches!(element.closest(INTERACTIVE_SELECTOR), Ok(Some(_))));
        if !interactive {
            if event.ctrl_key() || event.meta_key() || event.shift_key() {
                toggle.run(());
            } else {
                selected.set(HashSet::from([node_id]));
            }
        }
    };
    let checkbox_icon = move || {
        if is_selected() {
            ICON_CHECKED
        } else {
            ICON_UNCHECKED
        }
    };
    let checkbox_state = move || is_selected().to_string();
    let checkbox_label = format!("Select {}", item.node.name);

    view! {
        <tr
            class="node-table__row"
            class:node-table__row--selected=is_selected
            on:click=select
        >
            <td class="node-table__cell node-table__cell--select">
                <Checkbox
                    icon=Signal::derive(checkbox_icon)
                    checked=Signal::derive(checkbox_state)
                    label=checkbox_label
                    on_toggle=toggle
                />
            </td>
            <td class="node-table__cell">
                <span class="node-table__name">
                    <Icon name=icon />
                    {item.node.name.clone()}
                </span>
            </td>
            <td class="node-table__cell node-table__cell--modified">{item.original_path}</td>
            <td class="node-table__cell node-table__cell--modified">
                {short_date(item.deleted_at)}
            </td>
            <td class="node-table__cell node-table__cell--size">{human_size(item.node.size)}</td>
        </tr>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_counts_agree_with_the_number() {
        assert_eq!(files(1), "1 file");
        assert_eq!(files(3), "3 files");
    }
}
