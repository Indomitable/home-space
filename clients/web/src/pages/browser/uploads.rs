use std::rc::Rc;

use leptos::{prelude::*, task::spawn_local};
use web_sys::File;

use protocol::NodeId;

use crate::{
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
pub struct UploadItem {
    id: u64,
    name: String,
    progress: f64,
    state: UploadState,
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

pub fn start_upload(
    uploads: RwSignal<Vec<UploadItem>>,
    reload: RwSignal<u32>,
    auth: Auth,
    parent: Option<NodeId>,
    files: Vec<File>,
) {
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
}

#[component]
pub fn UploadList(uploads: RwSignal<Vec<UploadItem>>) -> impl IntoView {
    view! {
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
    }
}
