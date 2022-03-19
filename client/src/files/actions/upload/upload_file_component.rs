use js_sys::Array;
use log::debug;
use yew::prelude::*;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{DragEvent, DataTransferItem};

use super::file_system_api::{uploadDataTransferItems, showDirectoryPicker, showOpenFilePicker};

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
    SelectFiles,
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
            if let Ok(items) = showOpenFilePicker().await {
                debug!("{:?}", items);
                return FileUploadMessages::SelectFiles
            }
            return FileUploadMessages::None
        });

        let on_select_folder = ctx.link().callback_future(|_mouse_event| async {
            if let Ok(items) = showDirectoryPicker().await {
                debug!("{:?}", items);
                return FileUploadMessages::SelectFiles
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
        match msg {
            FileUploadMessages::DragLeave => {
                self.state = FileUploadState::None;
                true
            },
            FileUploadMessages::DragDrop(items) => {
                let parent_id = ctx.props().parent_id;
                // let callback = ctx.link().callback(|item: UploadProgress| {
                //     FileUploadMessages::UploadProgress(item)
                // });

                ctx.link().send_future(async move {
                    let _ = uploadDataTransferItems(parent_id, items).await;
                    FileUploadMessages::UploadFinish
                });
                self.state = FileUploadState::UploadingFiles;
                true
            },
            // FileUploadMessages::UploadProgress(item) => {
            //     debug!("{:?}", item);
            //     false
            // },
            FileUploadMessages::SelectFiles => {
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
