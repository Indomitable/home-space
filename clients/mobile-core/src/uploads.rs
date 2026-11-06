//! Files the user picks to upload. The app first copies each one to a file of its own, because a
//! picked document has no lasting path on either platform, and removes that copy afterwards.
//!
//! An upload that fails stays open on the server, so trying the same `key` again continues it.

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use api_client::{ApiError, Client};
use protocol::{CreateUpload, UploadSession};
use tokio::sync::watch;
use uuid::Uuid;

use crate::{CoreClient, CoreError, Entry, RUNTIME, off_ui_thread, parse_id};

const STOPPED: &str = "the upload was cancelled";

/// Told how far an upload has come.
///
/// Called on a background thread, so hand the numbers to the UI thread. It must not throw.
#[uniffi::export(with_foreign)]
pub trait UploadListener: Send + Sync {
    /// The server has `done` of the `total` bytes of the upload `key`.
    fn progress(&self, key: String, done: u64, total: u64);
}

/// The uploads running now, and the sessions that a retry can continue.
#[derive(Default)]
pub(crate) struct PickedUploads {
    running: Mutex<HashMap<String, watch::Sender<bool>>>,
    sessions: Mutex<HashMap<String, UploadSession>>,
}

impl PickedUploads {
    fn saved(&self, key: &str) -> Option<UploadSession> {
        self.sessions
            .lock()
            .expect("upload sessions lock")
            .get(key)
            .copied()
    }

    fn forget(&self, key: &str) -> Option<UploadSession> {
        self.sessions
            .lock()
            .expect("upload sessions lock")
            .remove(key)
    }

    /// Picks up where the last try of `key` stopped, or starts anew if the server let it go.
    async fn session(
        &self,
        client: &Client,
        key: &str,
        request: &CreateUpload,
    ) -> Result<UploadSession, ApiError> {
        if let Some(saved) = self.saved(key) {
            match client.upload_status(saved.id).await {
                Ok(status) => return Ok(status),
                Err(err) if err.is_transient() || err == ApiError::Unauthorized => return Err(err),
                Err(_) => {}
            }
        }
        let session = client.create_upload(request).await?;
        self.sessions
            .lock()
            .expect("upload sessions lock")
            .insert(key.to_owned(), session);
        Ok(session)
    }

    /// Stops everything, and returns the sessions to end on the server.
    pub(crate) fn stop_all(&self) -> Vec<Uuid> {
        for cancel in self.running.lock().expect("uploads lock").values() {
            cancel.send_replace(true);
        }
        self.sessions
            .lock()
            .expect("upload sessions lock")
            .drain()
            .map(|(_, session)| session.id)
            .collect()
    }
}

/// Marks `key` as running until dropped.
struct Running {
    uploads: Arc<PickedUploads>,
    key: String,
    cancelled: watch::Receiver<bool>,
}

impl Running {
    fn start(uploads: &Arc<PickedUploads>, key: &str) -> Result<Self, CoreError> {
        let mut running = uploads.running.lock().expect("uploads lock");
        if running.contains_key(key) {
            return Err(CoreError::Failed {
                code: "busy".to_owned(),
                detail: "this file is already uploading".to_owned(),
            });
        }
        let (cancel, cancelled) = watch::channel(false);
        running.insert(key.to_owned(), cancel);
        Ok(Self {
            uploads: uploads.clone(),
            key: key.to_owned(),
            cancelled,
        })
    }

    fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.uploads
            .running
            .lock()
            .expect("uploads lock")
            .remove(&self.key);
    }
}

#[uniffi::export]
impl CoreClient {
    /// Uploads the file at `path` into `parent_id`, or Home when `None`, as a new version if a
    /// file already has this name. `key` names this upload for `listener`, for
    /// [`CoreClient::cancel_picked_upload`] and for trying again after a failure, which needs the
    /// file to still be at `path`.
    pub async fn upload_picked_file(
        &self,
        key: String,
        parent_id: Option<String>,
        name: String,
        path: String,
        listener: Arc<dyn UploadListener>,
    ) -> Result<Entry, CoreError> {
        let parent = parent_id.as_deref().map(parse_id).transpose()?;
        let client = self.inner.clone();
        let uploads = self.picked_uploads.clone();
        off_ui_thread(async move {
            let running = Running::start(&uploads, &key)?;
            let path = Path::new(&path);
            let size = tokio::fs::metadata(path)
                .await
                .map_err(|err| CoreError::Failed {
                    code: "file".to_owned(),
                    detail: format!("the file can't be read: {err}"),
                })?
                .len();
            let request = CreateUpload {
                parent_id: parent,
                name,
                size,
                modified_at: None,
                mime: None,
                node_id: None,
                keep_both: false,
                auto_upload: false,
            };
            let session = uploads.session(&client, &key, &request).await?;
            let stored = client
                .continue_upload(
                    session,
                    size,
                    |offset, len| {
                        let stopped = running.is_cancelled();
                        async move {
                            if stopped {
                                return Err(io::Error::other(STOPPED));
                            }
                            sync_core::read_range(path, offset, len).await
                        }
                    },
                    |done| listener.progress(key.clone(), done, size),
                )
                .await;
            match stored {
                Ok(node) => {
                    uploads.forget(&key);
                    Ok(node.into())
                }
                Err(_) if running.is_cancelled() => {
                    uploads.forget(&key);
                    let _ = client.cancel_upload(session.id).await;
                    Err(CoreError::Cancelled)
                }
                Err(err) => Err(err.into()),
            }
        })
        .await
    }

    /// Stops the upload `key`, whose call then fails as `Cancelled`, or lets go of one that failed.
    pub fn cancel_picked_upload(&self, key: String) {
        let running = self.picked_uploads.running.lock().expect("uploads lock");
        if let Some(cancel) = running.get(&key) {
            cancel.send_replace(true);
        } else if let Some(session) = self.picked_uploads.forget(&key) {
            let client = self.inner.clone();
            RUNTIME.spawn(async move {
                let _ = client.cancel_upload(session.id).await;
            });
        }
    }
}
