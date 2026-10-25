use std::rc::Rc;

use leptos::{html::Input, prelude::*, task::spawn_local};
use leptos_router::{
    components::{A, Redirect},
    hooks::use_params_map,
};
use web_sys::{DragEvent, File, FileList, HtmlInputElement, KeyboardEvent};

const KEY_ENTER: &str = "Enter";
const KEY_ESCAPE: &str = "Escape";

use api_client::ApiError;
use protocol::{Event, Node, NodeId, NodeKind, TicketRequest};

use super::move_dialog::MoveDialog;
use crate::{
    events,
    format::{human_size, short_date},
    listing::{affects_path, apply_event, sort_nodes},
    session::{self, Auth},
    upload,
};

#[derive(Clone, PartialEq)]
enum UploadState {
    Queued,
    Active,
    Done,
    Failed(String),
}

#[derive(Clone)]
struct UploadItem {
    id: u64,
    name: String,
    progress: f64,
    state: UploadState,
}

#[component]
pub fn Browser(auth: Auth) -> impl IntoView {
    view! {
        <Show
            when=move || auth.user.get().is_some()
            fallback=|| view! { <Redirect path="/login" /> }
        >
            <Explorer auth />
        </Show>
    }
}

async fn fetch(folder: Option<NodeId>) -> Result<(Vec<Node>, Vec<Node>), ApiError> {
    let client = session::client();
    let mut listing = client.list(folder).await?;
    sort_nodes(&mut listing);
    let path = match folder {
        Some(id) => {
            let mut path = client.ancestors(id).await?;
            path.push(client.get(id).await?);
            path
        }
        None => Vec::new(),
    };
    Ok((listing, path))
}

fn confirm(message: &str) -> bool {
    web_sys::window()
        .and_then(|window| window.confirm_with_message(message).ok())
        .unwrap_or(false)
}

fn files_of(list: &FileList) -> Vec<File> {
    (0..list.length())
        .filter_map(|index| list.get(index))
        .collect()
}

fn update_upload(
    uploads: RwSignal<Vec<UploadItem>>,
    id: u64,
    change: impl FnOnce(&mut UploadItem),
) {
    uploads.update(|items| {
        if let Some(item) = items.iter_mut().find(|item| item.id == id) {
            change(item);
        }
    });
}

#[component]
fn Explorer(auth: Auth) -> impl IntoView {
    let params = use_params_map();
    let folder = Memo::new(move |_| {
        params
            .read()
            .get("id")
            .and_then(|id| id.parse::<NodeId>().ok())
    });
    let nodes = RwSignal::new(Vec::<Node>::new());
    let crumbs = RwSignal::new(Vec::<Node>::new());
    let error = RwSignal::new(None::<String>);
    let reload = RwSignal::new(0_u32);
    let moving = RwSignal::new(None::<Node>);
    let uploads = RwSignal::new(Vec::<UploadItem>::new());
    let draft = RwSignal::new(None::<String>);
    let renaming = RwSignal::new(None::<(NodeId, String)>);
    let saving = RwSignal::new(false);
    let draft_input = NodeRef::<Input>::new();

    Effect::new(move |_| {
        let wanted = folder.get();
        reload.track();
        spawn_local(async move {
            match fetch(wanted).await {
                Ok((listing, path)) if folder.get_untracked() == wanted => {
                    nodes.set(listing);
                    crumbs.set(path);
                    error.set(None);
                }
                Ok(_) => {}
                Err(err) => error.set(Some(auth.fail(&err))),
            }
        });
    });

    let connection = StoredValue::new_local(events::connect(move |event| {
        if matches!(event, Event::Resync) || affects_path(&crumbs.get_untracked(), &event) {
            reload.update(|count| *count += 1);
        } else {
            nodes.update(|listing| apply_event(listing, folder.get_untracked(), &event));
        }
    }));
    on_cleanup(move || connection.with_value(events::Connection::stop));

    let settle = move |result: Result<(), ApiError>| {
        if let Err(err) = result {
            error.set(Some(auth.fail(&err)));
        }
        reload.update(|count| *count += 1);
    };

    Effect::new(move |_| {
        folder.track();
        draft.set(None);
        renaming.set(None);
    });
    Effect::new(move |_| {
        if let Some(input) = draft_input.get() {
            let _ = input.focus();
        }
    });

    let new_folder = move |_| {
        if draft.get_untracked().is_none() {
            draft.set(Some(String::new()));
        } else if let Some(input) = draft_input.get_untracked() {
            let _ = input.focus();
        }
    };
    let cancel_folder = move || draft.set(None);
    let create_folder = move || {
        let name = draft.get_untracked().unwrap_or_default().trim().to_owned();
        if name.is_empty() {
            cancel_folder();
        } else if !saving.get_untracked() {
            saving.set(true);
            let parent = folder.get_untracked();
            spawn_local(async move {
                let result = session::client()
                    .create_folder(parent, &name)
                    .await
                    .map(drop);
                if result.is_ok() {
                    draft.set(None);
                }
                saving.set(false);
                settle(result);
            });
        }
    };
    let cancel_rename = move || renaming.set(None);
    let save_rename = move || {
        let Some((id, name)) = renaming.get_untracked() else {
            return;
        };
        let name = name.trim().to_owned();
        let unchanged = nodes
            .read_untracked()
            .iter()
            .any(|node| node.id == id && node.name == name);
        if name.is_empty() || unchanged {
            cancel_rename();
        } else if !saving.get_untracked() {
            saving.set(true);
            spawn_local(async move {
                let result = session::client().rename(id, &name).await.map(drop);
                if result.is_ok() {
                    renaming.set(None);
                }
                saving.set(false);
                settle(result);
            });
        }
    };
    let delete = move |node: Node| {
        if confirm(&format!("Delete “{}”? This cannot be undone.", node.name)) {
            spawn_local(async move {
                settle(session::client().delete(node.id).await);
            });
        }
    };
    let download = move |node: Node| {
        spawn_local(async move {
            let client = session::client();
            let request = TicketRequest::Download { node_id: node.id };
            match client.ticket(&request).await {
                Ok(ticket) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window
                            .location()
                            .set_href(&client.download_url(node.id, &ticket.ticket));
                    }
                }
                Err(err) => error.set(Some(auth.fail(&err))),
            }
        });
    };

    let start_upload = move |files: Vec<File>| {
        let parent = folder.get_untracked();
        let ids: Vec<u64> = files.iter().map(|_| upload::next_id()).collect();
        uploads.update(|items| {
            items.extend(ids.iter().zip(&files).map(|(id, file)| UploadItem {
                id: *id,
                name: file.name(),
                progress: 0.0,
                state: UploadState::Queued,
            }));
        });
        spawn_local(async move {
            for (id, file) in ids.into_iter().zip(files) {
                update_upload(uploads, id, |item| item.state = UploadState::Active);
                let on_progress: upload::OnProgress = Rc::new(move |fraction| {
                    update_upload(uploads, id, |item| item.progress = fraction)
                });
                match upload::upload(&session::client(), &file, parent, &on_progress).await {
                    Ok(_) => update_upload(uploads, id, |item| {
                        item.progress = 1.0;
                        item.state = UploadState::Done;
                    }),
                    Err(err) => {
                        let message = auth.fail(&err);
                        update_upload(uploads, id, |item| {
                            item.state = UploadState::Failed(message)
                        });
                    }
                }
            }
            reload.update(|count| *count += 1);
        });
    };

    view! {
        <header class="app-bar">
            <h1 class="app-bar__brand">"home-space"</h1>
            <span class="app-bar__spacer"></span>
            <span class="app-bar__user">{move || auth.user.get().map(|user| user.username)}</span>
            <button class="button button--ghost" on:click=move |_| auth.logout()>
                "Log out"
            </button>
        </header>
        <main
            class="explorer"
            on:dragover=|event: DragEvent| event.prevent_default()
            on:drop=move |event: DragEvent| {
                event.prevent_default();
                if let Some(list) = event.data_transfer().and_then(|transfer| transfer.files()) {
                    start_upload(files_of(&list));
                }
            }
        >
            <div class="explorer__header">
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
                                    href=format!("/folder/{}", folder.id)
                                    exact=true
                                    attr:class="breadcrumbs__link"
                                >
                                    {folder.name.clone()}
                                </A>
                            }
                        }
                    />
                </nav>
                <div class="explorer__toolbar">
                    <button class="button" on:click=new_folder>
                        "New folder"
                    </button>
                    <label class="button button--primary">
                        "Upload files"
                        <input
                            type="file"
                            multiple
                            hidden
                            on:change:target=move |event| {
                                let input: HtmlInputElement = event.target();
                                if let Some(list) = input.files() {
                                    start_upload(files_of(&list));
                                }
                                input.set_value("");
                            }
                        />
                    </label>
                </div>
            </div>
            {move || {
                error
                    .get()
                    .map(|message| {
                        view! { <p class="alert alert--error" role="alert">{message}</p> }
                    })
            }}
            <Show when=move || !uploads.read().is_empty()>
                <ul class="upload-list">
                    {move || {
                        uploads
                            .get()
                            .into_iter()
                            .map(|item| {
                                let (status, modifier) = match &item.state {
                                    UploadState::Queued => ("Waiting".to_owned(), "queued"),
                                    UploadState::Active => ("Uploading".to_owned(), "active"),
                                    UploadState::Done => ("Done".to_owned(), "done"),
                                    UploadState::Failed(message) => {
                                        (format!("Failed: {message}"), "failed")
                                    }
                                };
                                view! {
                                    <li class=format!(
                                        "upload-list__item upload-list__item--{modifier}",
                                    )>
                                        <span class="upload-list__name">{item.name}</span>
                                        <progress
                                            class="upload-list__progress"
                                            max="1"
                                            value=item.progress
                                        ></progress>
                                        <span class="upload-list__status">{status}</span>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </Show>
            <table class="node-table">
                <thead class="node-table__head">
                    <tr>
                        <th class="node-table__header-cell">"Name"</th>
                        <th class="node-table__header-cell node-table__header-cell--size">
                            "Size"
                        </th>
                        <th class="node-table__header-cell node-table__header-cell--modified">
                            "Modified"
                        </th>
                        <th class="node-table__header-cell">
                            <span class="visually-hidden">"Actions"</span>
                        </th>
                    </tr>
                </thead>
                <tbody class="node-table__body">
                    <Show when=move || draft.read().is_some()>
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
                                        on:input:target=move |event| {
                                            draft.set(Some(event.target().value()))
                                        }
                                        on:keydown=move |event: KeyboardEvent| {
                                            match event.key().as_str() {
                                                KEY_ENTER => create_folder(),
                                                KEY_ESCAPE => cancel_folder(),
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
                                        on:click=move |_| create_folder()
                                    >
                                        "Create"
                                    </button>
                                    <button
                                        class="button button--ghost button--small"
                                        disabled=move || saving.get()
                                        on:click=move |_| cancel_folder()
                                    >
                                        "Cancel"
                                    </button>
                                </div>
                            </td>
                        </tr>
                    </Show>
                    <For
                        each=move || nodes.get()
                        key=|node| (node.id, node.updated_at.unix_timestamp_nanos())
                        children=move |node| {
                            let is_folder = node.kind == NodeKind::Folder;
                            let name = node.name.clone();
                            let size = if is_folder {
                                String::new()
                            } else {
                                human_size(node.size)
                            };
                            let modified = short_date(node.updated_at);
                            let href = format!("/folder/{}", node.id);
                            let node_id = node.id;
                            let editing = move || {
                                renaming.read().as_ref().is_some_and(|(id, _)| *id == node_id)
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
                                                                KEY_ENTER => save_rename(),
                                                                KEY_ESCAPE => cancel_rename(),
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
                                                <A
                                                    href=href
                                                    attr:class="node-table__name node-table__name--link"
                                                >
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
                                    <td class="node-table__cell node-table__cell--modified">
                                        {modified}
                                    </td>
                                    <td class="node-table__cell node-table__cell--actions">
                                        {move || if editing() {
                                            view! {
                                                <div class="node-table__actions node-table__actions--editing">
                                                    <button
                                                        class="button button--primary button--small"
                                                        disabled=move || saving.get()
                                                        on:click=move |_| save_rename()
                                                    >
                                                        "Save"
                                                    </button>
                                                    <button
                                                        class="button button--ghost button--small"
                                                        disabled=move || saving.get()
                                                        on:click=move |_| cancel_rename()
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
                                                            on:click=move |_| {
                                                                download(for_download.clone())
                                                            }
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
                                                on:click=move |_| {
                                                    moving.set(Some(for_move.clone()))
                                                }
                                            >
                                                "Move"
                                            </button>
                                            <button
                                                class="button button--ghost button--small button--danger"
                                                on:click=move |_| delete(for_delete.clone())
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
                    />
                </tbody>
            </table>
            <Show when=move || nodes.read().is_empty() && error.read().is_none() && draft.read().is_none()>
                <p class="explorer__empty">"Nothing here yet. Drop files anywhere to upload."</p>
            </Show>
        </main>
        {move || {
            moving.get().map(|node| view! { <MoveDialog node moving reload auth /> })
        }}
    }
}
