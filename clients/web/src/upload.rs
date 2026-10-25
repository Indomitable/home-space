use std::{cell::Cell, cell::RefCell, rc::Rc};

use tokio::sync::oneshot;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{File, ProgressEvent, XmlHttpRequest};

use api_client::{ApiError, Client};
use protocol::{ErrorBody, Node, NodeId, paths};

const STATUS_CREATED: u16 = 201;
const STATUS_UNAUTHORIZED: u16 = 401;

thread_local! {
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
}

/// A fresh identifier for an upload row.
pub fn next_id() -> u64 {
    NEXT_ID.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    })
}

/// Progress callback receiving a fraction between 0 and 1.
pub type OnProgress = Rc<dyn Fn(f64)>;

struct Outcome {
    status: u16,
    body: String,
}

/// Uploads `file` into `parent` with progress, refreshing the session once on a 401.
pub async fn upload(
    client: &Client,
    file: &File,
    parent: Option<NodeId>,
    on_progress: &OnProgress,
) -> Result<Node, ApiError> {
    let token = client.access_token().ok_or(ApiError::Unauthorized)?;
    let mut outcome = send(client, &token, file, parent, on_progress).await?;
    if outcome.status == STATUS_UNAUTHORIZED {
        client.refresh_session(&token).await?;
        let token = client.access_token().ok_or(ApiError::Unauthorized)?;
        outcome = send(client, &token, file, parent, on_progress).await?;
    }
    if outcome.status == STATUS_CREATED {
        serde_json::from_str(&outcome.body).map_err(|err| ApiError::Decode(err.to_string()))
    } else {
        Err(failure(&outcome))
    }
}

fn failure(outcome: &Outcome) -> ApiError {
    if outcome.status == STATUS_UNAUTHORIZED {
        ApiError::Unauthorized
    } else {
        match serde_json::from_str::<ErrorBody>(&outcome.body) {
            Ok(body) => ApiError::Api {
                code: body.error,
                message: body.message,
            },
            Err(_) => ApiError::Transport(format!("upload failed with status {}", outcome.status)),
        }
    }
}

fn transport(err: JsValue) -> ApiError {
    ApiError::Transport(format!("{err:?}"))
}

async fn send(
    client: &Client,
    token: &str,
    file: &File,
    parent: Option<NodeId>,
    on_progress: &OnProgress,
) -> Result<Outcome, ApiError> {
    let name = String::from(js_sys::encode_uri_component(&file.name()));
    let mut url = format!("{}?name={name}", client.url(paths::FILES));
    if let Some(parent) = parent {
        url.push_str(&format!("&parent_id={parent}"));
    }
    let xhr = XmlHttpRequest::new().map_err(transport)?;
    xhr.open_with_async("POST", &url, true).map_err(transport)?;
    xhr.set_request_header("Authorization", &format!("Bearer {token}"))
        .map_err(transport)?;

    let (done_tx, done) = oneshot::channel::<bool>();
    let done_tx = Rc::new(RefCell::new(Some(done_tx)));
    let finish = |ok: bool| {
        let done_tx = Rc::clone(&done_tx);
        Closure::<dyn FnMut()>::new(move || {
            if let Some(tx) = done_tx.borrow_mut().take() {
                let _ = tx.send(ok);
            }
        })
    };
    let on_load = finish(true);
    let on_error = finish(false);
    let progress = Rc::clone(on_progress);
    let on_upload_progress =
        Closure::<dyn FnMut(ProgressEvent)>::new(move |event: ProgressEvent| {
            if event.length_computable() && event.total() > 0.0 {
                progress(event.loaded() / event.total());
            }
        });
    xhr.set_onload(Some(on_load.as_ref().unchecked_ref()));
    xhr.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    xhr.set_onabort(Some(on_error.as_ref().unchecked_ref()));
    xhr.upload()
        .map_err(transport)?
        .set_onprogress(Some(on_upload_progress.as_ref().unchecked_ref()));
    xhr.send_with_opt_blob(Some(file)).map_err(transport)?;

    match done.await {
        Ok(true) => Ok(Outcome {
            status: xhr.status().map_err(transport)?,
            body: xhr.response_text().map_err(transport)?.unwrap_or_default(),
        }),
        _ => Err(ApiError::Transport(
            "network error during upload".to_owned(),
        )),
    }
}
