use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::{modal::modal_dialog::{ModalDialog, ModalDialogHeader}};
use super::file_api::defineFileUpload;

#[derive(Properties, PartialEq)]
pub struct UploadFileProps {
    pub onclick: Callback<()>
}

#[function_component(UploadFileAction)]
pub fn upload_file_action(props: &UploadFileProps) -> Html {
    let upload_file_modal_open = use_state(|| false);

    let onclick = {
        let upload_file_modal_open = upload_file_modal_open.clone();
        // let on_click = props.onclick.clone();
        // Switch to input on click
        Callback::from(move |_| {
            // spawn_local(async {
            //     get_directory().await
            // });

            //defineFileUpload();
            upload_file_modal_open.set(true);
        })
    };

    let on_backdrop_click = {
        let upload_file_modal_open = upload_file_modal_open.clone();
        // Switch to input on click
        Callback::from(move |_| {
            upload_file_modal_open.set(false);                
        })
    };

    let modal_dilog_header = ModalDialogHeader::Text("Header".to_owned());

    html!{
        <>
            <a {onclick}>
                <span class="icon-outlined">{"upload_file"}</span>
                <span>{"Upload file"}</span>
            </a>

            if *upload_file_modal_open {
                <ModalDialog header={modal_dilog_header} use_backdrop={Some(true)} on_backdrop_click={Some(on_backdrop_click)}>
                    <file-upload></file-upload>
                </ModalDialog>
            }
        </>
    }
}