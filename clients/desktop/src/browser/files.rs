use std::path::PathBuf;

use iced::Task;

use api_client::Client;
use protocol::{Node, NodeId, NodeKind};

use super::Message;
use crate::transfers::{self, DownloadFile, UploadPlan, sanitize_component};

const TEMP_FOLDER: &str = "home-space";

/// Asks for files to upload into `parent` and plans their transfer.
pub fn pick_upload(parent: Option<NodeId>) -> Task<Message> {
    Task::perform(
        async move {
            match rfd::AsyncFileDialog::new().pick_files().await {
                Some(handles) => {
                    let paths = handles
                        .iter()
                        .map(|handle| handle.path().to_path_buf())
                        .collect();
                    transfers::plan_upload(paths).await
                }
                None => Ok(UploadPlan::default()),
            }
        },
        move |result| Message::UploadPlanned(parent, result),
    )
}

/// Plans the upload of dropped `paths` into `parent`.
pub fn plan_upload(parent: Option<NodeId>, paths: Vec<PathBuf>) -> Task<Message> {
    Task::perform(transfers::plan_upload(paths), move |result| {
        Message::UploadPlanned(parent, result)
    })
}

/// Asks where to save `nodes` and plans their download.
pub fn download(client: &Client, nodes: Vec<Node>) -> Task<Message> {
    match nodes.as_slice() {
        [node] if node.kind == NodeKind::File => {
            let node = node.clone();
            Task::perform(
                async move {
                    let handle = rfd::AsyncFileDialog::new()
                        .set_file_name(&node.name)
                        .save_file()
                        .await;
                    Ok(handle
                        .map(|handle| {
                            vec![DownloadFile::replacing(&node, handle.path().to_path_buf())]
                        })
                        .unwrap_or_default())
                },
                Message::DownloadPlanned,
            )
        }
        _ => {
            let client = client.clone();
            Task::perform(
                async move {
                    match rfd::AsyncFileDialog::new().pick_folder().await {
                        Some(folder) => {
                            transfers::plan_download(client, nodes, folder.path().to_path_buf())
                                .await
                        }
                        None => Ok(Vec::new()),
                    }
                },
                Message::DownloadPlanned,
            )
        }
    }
}

/// Where `node` is downloaded to be opened: a folder of its own in the system's temp folder, so
/// the file keeps its name and the system picks the app by its extension.
pub fn temp_target(node: &Node) -> PathBuf {
    std::env::temp_dir()
        .join(TEMP_FOLDER)
        .join(node.id.0.to_string())
        .join(sanitize_component(&node.name))
}

/// Downloads the file `node` to the temp folder and opens it with the default app.
pub fn download_and_open(node: Node) -> Task<Message> {
    let target = temp_target(&node);
    Task::perform(
        async move {
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|err| format!("{}: {err}", parent.display()))?;
            }
            Ok(vec![DownloadFile::opening(&node, target)])
        },
        Message::DownloadPlanned,
    )
}
