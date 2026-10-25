mod app_bar;
mod breadcrumbs;
mod node_table;
mod toolbar;

use leptos::{ev, html::Input, prelude::*, task::spawn_local};
use leptos_router::{components::Redirect, hooks::use_params_map};
use web_sys::{DragEvent, File, FileList};

use api_client::ApiError;
use protocol::{Event, Node, NodeId, TicketRequest};

use self::{
    app_bar::AppBar,
    breadcrumbs::Breadcrumbs,
    node_table::{NodeTable, TableActions},
    toolbar::Toolbar,
};
use super::move_dialog::MoveDialog;
use crate::{
    events,
    listing::{affects_path, apply_event, sort_nodes},
    route,
    session::{self, Auth},
    uploads::{UploadItem, start_upload},
};

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

const FILES_TYPE: &str = "Files";
const DROP_EFFECT_COPY: &str = "copy";

fn carries_files(event: &DragEvent) -> bool {
    event
        .data_transfer()
        .is_some_and(|transfer| transfer.types().includes(&FILES_TYPE.into(), 0))
}

fn files_of(list: &FileList) -> Vec<File> {
    (0..list.length())
        .filter_map(|index| list.get(index))
        .collect()
}

#[component]
fn Explorer(auth: Auth) -> impl IntoView {
    let params = use_params_map();
    let folder = Memo::new(move |_| params.read().get("id").and_then(|id| route::parse_id(&id)));
    let nodes = RwSignal::new(Vec::<Node>::new());
    let crumbs = RwSignal::new(Vec::<Node>::new());
    let error = RwSignal::new(None::<String>);
    let reload = RwSignal::new(0_u32);
    let moving = RwSignal::new(None::<Node>);
    let uploads = expect_context::<RwSignal<Vec<UploadItem>>>();
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

    let new_folder = Callback::new(move |()| {
        if draft.get_untracked().is_none() {
            draft.set(Some(String::new()));
        } else if let Some(input) = draft_input.get_untracked() {
            let _ = input.focus();
        }
    });
    let cancel_folder = Callback::new(move |()| draft.set(None));
    let create_folder = Callback::new(move |()| {
        let name = draft.get_untracked().unwrap_or_default().trim().to_owned();
        if name.is_empty() {
            cancel_folder.run(());
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
    });
    let cancel_rename = Callback::new(move |()| renaming.set(None));
    let save_rename = Callback::new(move |()| {
        let Some((id, name)) = renaming.get_untracked() else {
            return;
        };
        let name = name.trim().to_owned();
        let unchanged = nodes
            .read_untracked()
            .iter()
            .any(|node| node.id == id && node.name == name);
        if name.is_empty() || unchanged {
            cancel_rename.run(());
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
    });
    let delete = Callback::new(move |node: Node| {
        if confirm(&format!("Delete “{}”? This cannot be undone.", node.name)) {
            spawn_local(async move {
                settle(session::client().delete(node.id).await);
            });
        }
    });
    let download = Callback::new(move |node: Node| {
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
    });
    let upload = Callback::new(move |files: Vec<File>| {
        start_upload(uploads, reload, auth, folder.get_untracked(), files);
    });

    let drag_depth = RwSignal::new(0_u32);
    let dragging = Memo::new(move |_| drag_depth.get() > 0);
    let on_enter = window_event_listener(ev::dragenter, move |event| {
        if carries_files(&event) {
            event.prevent_default();
            drag_depth.update(|depth| *depth += 1);
        }
    });
    let on_over = window_event_listener(ev::dragover, move |event| {
        if carries_files(&event) {
            event.prevent_default();
            if let Some(transfer) = event.data_transfer() {
                transfer.set_drop_effect(DROP_EFFECT_COPY);
            }
        }
    });
    let on_leave = window_event_listener(ev::dragleave, move |event| {
        if carries_files(&event) {
            drag_depth.update(|depth| *depth = depth.saturating_sub(1));
        }
    });
    let on_drop = window_event_listener(ev::drop, move |event| {
        event.prevent_default();
        drag_depth.set(0);
        if let Some(list) = event.data_transfer().and_then(|transfer| transfer.files()) {
            upload.run(files_of(&list));
        }
    });
    on_cleanup(move || {
        on_enter.remove();
        on_over.remove();
        on_leave.remove();
        on_drop.remove();
    });

    let actions = TableActions {
        draft,
        renaming,
        saving,
        moving,
        create_folder,
        cancel_folder,
        save_rename,
        cancel_rename,
        download,
        delete,
    };

    view! {
        <div class="browser">
            <AppBar auth />
            <main class="explorer">
                <div class="explorer__header">
                    <Breadcrumbs crumbs />
                    <Toolbar on_new_folder=new_folder on_upload=upload />
                </div>
                {move || {
                    error
                        .get()
                        .map(|message| {
                            view! { <p class="alert alert--error" role="alert">{message}</p> }
                        })
                }}
                <div class="explorer__scroll">
                    <NodeTable nodes actions draft_input />
                    <Show when=move || nodes.read().is_empty() && error.read().is_none() && draft.read().is_none()>
                        <p class="explorer__empty">"Nothing here yet. Drop files anywhere to upload."</p>
                    </Show>
                </div>
            </main>
        </div>
        <Show when=move || dragging.get()>
            <div class="drop-overlay" aria-hidden="true">
                <p class="drop-overlay__label">
                    {move || {
                        let target = crumbs
                            .read()
                            .last()
                            .map_or_else(|| "Home".to_owned(), |node| node.name.clone());
                        format!("Drop files to upload to {target}")
                    }}
                </p>
            </div>
        </Show>
        {move || {
            moving.get().map(|node| view! { <MoveDialog node moving reload auth /> })
        }}
    }
}
