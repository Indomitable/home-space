use std::process::Output;

use js_sys::{Array, Promise};
use log::debug;
use wasm_bindgen::{JsValue, JsCast, UnwrapThrowExt, prelude::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use yew::prelude::*;
use web_sys::{DragEvent, DataTransferItemList, DataTransferItem, FileSystemEntry, FileSystemDirectoryEntry};
use gloo_events::EventListener;

use crate::{modal::modal_dialog::{ModalDialog, ModalDialogHeader}};

#[derive(Properties, PartialEq)]
pub struct UploadFileProps {
    pub parent_id: i64,
    pub supports_open_dialog: bool
}

pub enum UploadFileMessages {
    OpenModalDialog,
    CloseModalDialog
}

pub struct UploadFileAction {
    is_upload_file_modal_open: bool,
    on_uploaded: Option<EventListener>
}

impl Component for UploadFileAction {
    type Message = UploadFileMessages;
    type Properties = UploadFileProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            is_upload_file_modal_open: false,
            on_uploaded: None
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            UploadFileMessages::OpenModalDialog => {
                self.is_upload_file_modal_open = true;
                true
            },
            UploadFileMessages::CloseModalDialog => {
                self.is_upload_file_modal_open = false;
                true
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let onclick = ctx.link().callback(|_| UploadFileMessages::OpenModalDialog);

        let action = html! {
            <a {onclick}>
                <span class="icon-outlined">{"upload_file"}</span>
                <span>{"Upload file"}</span>
            </a>
        };

        if self.is_upload_file_modal_open {
            let modal_dilog_header = ModalDialogHeader::Text("Select file(s) to upload.".to_owned());
            let on_backdrop_click = ctx.link().callback(|_| UploadFileMessages::CloseModalDialog );

            let UploadFileProps { parent_id, supports_open_dialog } = ctx.props();
            
            return html!{
                <>
                    {action}
                    <ModalDialog header={modal_dilog_header} use_backdrop={Some(true)} on_backdrop_click={Some(on_backdrop_click)}>
                        <FileUpload parent_id={parent_id} supports_open_dialog={*supports_open_dialog} />
                    </ModalDialog>
                </>
            };
        }
        action
    }
}

struct FileUpload {
    state: FileUploadState
}

enum FileUploadState {
    None,
    DragOver,
    HasFiles
}

enum FileUploadMessages {
    DragLeave,
    DragDrop(Vec<DataTransferItem>),
    DragOver,
    None,
}

impl FileUpload {
    fn get_class_container(&self) -> String {
        let static_class = "file-upload__drop-container";
        let state_class = match self.state {
            FileUploadState::None => "",
            FileUploadState::DragOver => "file-upload__drag-over",
            FileUploadState::HasFiles => "file-upload__uploading",
        };
        format!("{} {}", static_class, state_class)
    }
}

async fn upload_files(parent_id: i64, files: Vec<DataTransferItem>) {
    for file in files {
        let entry = file.webkit_get_as_entry().unwrap_throw().unwrap_throw();
        upload_entry(parent_id, entry).await;
    }
}

async fn upload_entry(parent_id: i64, entry: FileSystemEntry) {
    if entry.is_file() {

    }

    if entry.is_directory() {
        let entries = read_directory_promise(entry.unchecked_into::<FileSystemDirectoryEntry>()).await;
        debug!("{:?}", entries);
    }
}

async fn read_directory_promise(entry: FileSystemDirectoryEntry) -> Vec<FileSystemEntry> {
    let reader = entry.create_reader();
    let promise = Promise::new(&mut |resolve, reject| {
        let success_callback: Closure<dyn FnMut(js_sys::Array)> = Closure::wrap(Box::new(move |entries: js_sys::Array| {
            debug!("{:?}", entries);
            resolve.apply(&JsValue::NULL, &entries).unwrap_throw();
        }) as Box<dyn FnMut(js_sys::Array)>);
        // let error_callback = |error: js_sys::Error| {

        // };
        reader.read_entries_with_file_system_entries_callback(success_callback.as_ref().unchecked_ref()).unwrap_throw();
    });
    let entries = JsFuture::from(promise).await.unwrap_throw();
    entries.dyn_into::<js_sys::Array>().unwrap_throw().to_vec().into_iter().map(|item| {
        debug!("{:?}", item);
        item.dyn_into::<FileSystemEntry>().unwrap_throw()
    }).collect()
    //reader.read_entries_with_callback(success_callback)
}

impl Component for FileUpload {
    type Message = FileUploadMessages;

    type Properties = UploadFileProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            state: FileUploadState::None
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let UploadFileProps { parent_id, supports_open_dialog } = ctx.props();

        let ondragleave = ctx.link().callback(|drag_event| FileUploadMessages::DragLeave);
        let is_drag_over = match self.state {
            FileUploadState::DragOver => true,
            _  => false
        };
        let ondrop = ctx.link().callback(move |drag_event: DragEvent| {
            if is_drag_over {
                let data_transfer = drag_event.data_transfer().unwrap_throw();
                let items: Array = Array::from(&data_transfer.items());
                if items.length() > 0 {                    
                    drag_event.prevent_default();
                    let vec = items.iter().map(|item| item.dyn_into::<DataTransferItem>().unwrap_throw()).collect();
                    return FileUploadMessages::DragDrop(vec);
                }
            }
            FileUploadMessages::None
        });
        let ondragover = ctx.link().callback(|drag_event: DragEvent| {
            if let Some(data_transfer) = drag_event.data_transfer() {
                let items: Array = Array::from(&data_transfer.items());
                if items.length() > 0 && items.every(&mut |item, _, _| { item.dyn_into::<DataTransferItem>().unwrap_throw().kind() == "file" }) {
                    drag_event.prevent_default();
                    return FileUploadMessages::DragOver
                }
            }
            return FileUploadMessages::None
        });

        html!{
            <div class={self.get_class_container()} {ondragleave} {ondrop} {ondragover}>
                <div class="drop-hint">
                    if *supports_open_dialog {
                        <span>{"Drag and drop file(s) or folder(s) you want to upload or click "}<a class="select-file">{"select"}</a></span>
                    } else {
                        <span>{"Drag and drop file(s) or folder(s) you want to upload or "}<input type="file" /></span>
                    }
                </div>
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            FileUploadMessages::DragLeave => {
                self.state = FileUploadState::None;
                true
            },
            FileUploadMessages::DragDrop(items) => {
                debug!("{:?}", items);
                let parent_id = ctx.props().parent_id;
                spawn_local(async move {
                    upload_files(parent_id, items).await;
                });
                self.state = FileUploadState::HasFiles;
                true
            },
            FileUploadMessages::DragOver => {
                self.state = FileUploadState::DragOver;
                true
            },
            FileUploadMessages::None => {
                false
            }
        }
    }
}