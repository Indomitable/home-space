use yew::prelude::*;
use gloo_events::EventListener;

use crate::{modal::modal_dialog::{ModalDialog, ModalDialogHeader}};

#[derive(Properties, PartialEq)]
pub struct UploadFileProps {
    pub parent_id: i64,
    pub supports_open_dialog: bool
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

    use_effect_with_deps(
        move |_| {
            let listener = gloo::events::EventListener::new(&span_ref.cast::<web_sys::HtmlElement>().unwrap(), "click", move |event| {
                web_sys::console::log_1(&"I got clicked".into());
            });
            move || drop(listener)
        },
        (),
    );


    html!{
        <>
            <a {onclick}>
                <span class="icon-outlined">{"upload_file"}</span>
                <span>{"Upload file"}</span>
            </a>

            if *upload_file_modal_open {
                { build_modal_dialog(props, on_backdrop_click) }
            }
        </>
    }
}

pub fn build_modal_dialog(props: &UploadFileProps, on_backdrop_click: Callback<MouseEvent>) -> Html {
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