use js_sys::Array;
use wasm_bindgen::{JsValue, JsCast, UnwrapThrowExt};
use yew::prelude::*;
use web_sys::{DragEvent, DataTransferItemList, DataTransferItem};
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
            
            let modal_body = gloo_utils::document().create_element("file-upload").unwrap();
            modal_body.set_attribute("parent-id", &parent_id.to_string()).unwrap();
            if *supports_open_dialog {
                modal_body.set_attribute("supports-open-dialog", "").unwrap();
            }

            let vref = Html::VRef(modal_body.into());



            return html!{
                <>
                    {action}
                    <ModalDialog header={modal_dilog_header} use_backdrop={Some(true)} on_backdrop_click={Some(on_backdrop_click)}>
                        {vref}
                    </ModalDialog>
                </>
            };
        }
        action
    }
}


// #[function_component(UploadFileAction)]
// pub fn upload_file_action(props: &UploadFileProps) -> Html {
//     let upload_file_modal_open = use_state(|| false);

//     let onclick = {
//         let upload_file_modal_open = upload_file_modal_open.clone();
//         // let on_click = props.onclick.clone();
//         // Switch to input on click
//         Callback::from(move |_| {
//             // spawn_local(async {
//             //     get_directory().await
//             // });

//             //defineFileUpload();
//             upload_file_modal_open.set(true);
//         })
//     };

//     let on_backdrop_click = {
//         let upload_file_modal_open = upload_file_modal_open.clone();
//         // Switch to input on click
//         Callback::from(move |_| {
//             upload_file_modal_open.set(false);                
//         })
//     };
//     let modal_dilog_header = ModalDialogHeader::Text("Select file(s) to upload.".to_owned());

//     html!{
//         <>
//             <a {onclick}>
//                 <span class="icon-outlined">{"upload_file"}</span>
//                 <span>{"Upload file"}</span>
//             </a>

//             if *upload_file_modal_open {
                

//                 // { build_modal_dialog(props, &file_upload_ref, on_backdrop_click) }
//                 <ModalDialog header={modal_dilog_header} use_backdrop={Some(true)} on_backdrop_click={Some(on_backdrop_click)}>
//                     {Html::VRef(modal_body.into())}
//                 </ModalDialog>
//             }
//         </>
//     }
// }

pub fn build_modal_dialog(props: &UploadFileProps, node_ref: &NodeRef, on_backdrop_click: Callback<MouseEvent>) -> Html {
    let modal_body = gloo_utils::document().create_element("file-upload").unwrap();
    modal_body.set_attribute("parent-id", &props.parent_id.to_string()).unwrap();
    if props.supports_open_dialog {
        modal_body.set_attribute("supports-open-dialog", "").unwrap();
    }
    let modal_dilog_header = ModalDialogHeader::Text("Select file(s) to upload.".to_owned());
    html!{
        <ModalDialog header={modal_dilog_header} use_backdrop={Some(true)} on_backdrop_click={Some(on_backdrop_click)}>
            {Html::VRef(modal_body.into())}
        </ModalDialog>
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
        let static_class = "drop-container";
        let state_class = match self.state {
            FileUploadState::None => "drop-container",
            FileUploadState::DragOver => "drop-container file-upload__drag-over",
            FileUploadState::HasFiles => "drop-container file-upload__uploading",
        };
        format!("{} {}", static_class, state_class)
    }
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
                    let vec = items.iter().map(|item| item.unchecked_into::<DataTransferItem>()).collect();
                    return FileUploadMessages::DragDrop(vec);
                }
            }
            FileUploadMessages::None
        });
        let ondragover = ctx.link().callback(|drag_event: DragEvent| {
            if let Some(data_transfer) = drag_event.data_transfer() {
                let items: Array = Array::from(&data_transfer.items());
                if items.length() > 0 && items.every(&mut |item, _, _| { item.unchecked_into::<DataTransferItem>().kind() == "file" }) {
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