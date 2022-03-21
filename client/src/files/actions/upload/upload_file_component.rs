use js_sys::Array;
use serde::Serialize;
use yew::prelude::*;
use wasm_bindgen::{JsCast, UnwrapThrowExt, JsValue};
use web_sys::{DragEvent, DataTransferItem};

use crate::files::{actions::upload::file_system_api::FileSystemFileHandle};

use super::file_system_api::{
    upload_data_transfer_items,
    upload_directory_handle,
    show_directory_picker,
    show_open_file_picker,
    upload_file
};

pub(crate) struct FileUpload {
    state: FileUploadState
}

#[derive(Properties, PartialEq)]
pub struct FileUploadProps {
    pub parent_id: i64,
    pub supports_open_dialog: bool,
    pub on_files_uploaded: Callback<()>
}

pub(crate) enum FileUploadState {
    None,
    DragOver,
    UploadingFiles
}


// #[derive(Deserialize, Debug)]
// pub(crate) struct UploadProgress {
//     pub(crate) ftype: i32,
//     pub(crate) name: String
// }

pub(crate) enum FileUploadMessages {
    None,
    DragOver,
    DragLeave,
    DragDrop(Array),
    // UploadProgress(UploadProgress),
    UploadFinish,
    SelectFiles(Vec<FileSystemFileHandle>),
    SelectDirectory(JsValue)
}

impl FileUpload {
    fn get_class_container(&self) -> String {
        let static_class = "file-upload__drop-container";
        let state_class = match self.state {
            FileUploadState::None => "",
            FileUploadState::DragOver => "file-upload__drag-over",
            FileUploadState::UploadingFiles => "file-upload__uploading",
        };
        format!("{} {}", static_class, state_class)
    }
}
impl Component for FileUpload {
    type Message = FileUploadMessages;

    type Properties = FileUploadProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            state: FileUploadState::None
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let supports_open_dialog = ctx.props().supports_open_dialog;

        let ondragleave = ctx.link().callback(|_drag_event| FileUploadMessages::DragLeave);
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
                    return FileUploadMessages::DragDrop(items);
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

        let on_select_files = ctx.link().callback_future(|_mouse_event| async {
            #[derive(Serialize)]
            struct OpenFilePickerOptions { multiple: bool }
            let options = OpenFilePickerOptions{ multiple: true };

            let options = JsValue::from_serde(&options).unwrap_throw();
            if let Ok(items) = show_open_file_picker(options).await {
                let file_handles = items.unchecked_into::<Array>().to_vec().into_iter().map(|value| { value.unchecked_into::<FileSystemFileHandle>() }).collect();
                return FileUploadMessages::SelectFiles(file_handles)
            }
            return FileUploadMessages::None
        });

        let on_select_folder = ctx.link().callback_future(|_mouse_event| async {
            if let Ok(directory) = show_directory_picker().await {
                return FileUploadMessages::SelectDirectory(directory);
            }
            return FileUploadMessages::None
        });

        html!{
            <div class={self.get_class_container()} {ondragleave} {ondrop} {ondragover}>
                <div class="drop-hint">
                    if supports_open_dialog {
                        <span>{"Drag and drop file(s) or folder(s) you want to upload or click "}<a class="select-file" onclick={on_select_files}>{"select"}</a>
                        {" to select files for upload or "}<br/><a class="select-folder" onclick={on_select_folder}>{"select"}</a>{" to select folder."}
                        </span>
                    } else {
                        <span>{"Drag and drop file(s) or folder(s) you want to upload or "}<input type="file" /></span>
                    }
                </div>
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let parent_id: JsValue = JsValue::from_str(&ctx.props().parent_id.to_string());
        match msg {
            FileUploadMessages::DragLeave => {
                self.state = FileUploadState::None;
                true
            },
            FileUploadMessages::DragDrop(items) => {
                // let callback = ctx.link().callback(|item: UploadProgress| {
                //     FileUploadMessages::UploadProgress(item)
                // });

                ctx.link().send_future(async move {
                    let _ = upload_data_transfer_items(parent_id, items).await;
                    FileUploadMessages::UploadFinish
                });
                self.state = FileUploadState::UploadingFiles;
                true
            },
            // FileUploadMessages::UploadProgress(item) => {
            //     debug!("{:?}", item);
            //     false
            // },
            FileUploadMessages::SelectFiles(file_handles) => {
                ctx.link().send_future(async move {
                    for file_handle in file_handles  {
                        let parent_id = parent_id.clone();
                        match file_handle.getFile().await {
                            Ok(file) => {
                                let _ = upload_file(parent_id, file).await.unwrap_throw();
                            },
                            Err(_) => {
                                // Skip file.
                            },
                        }
                    }
                    return FileUploadMessages::UploadFinish;
                });
                
                false
            },
            FileUploadMessages::SelectDirectory(directory) => {
                ctx.link().send_future(async move {
                    let _ = upload_directory_handle(parent_id, directory).await;
                    return FileUploadMessages::UploadFinish;
                });
                false
            },
            FileUploadMessages::DragOver => {
                self.state = FileUploadState::DragOver;
                true
            },
            FileUploadMessages::UploadFinish => {
                ctx.props().on_files_uploaded.emit(());
                false
            },
            FileUploadMessages::None => {
                false
            },
            
        }
    }
}
