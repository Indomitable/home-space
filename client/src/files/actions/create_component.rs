use std::ops::Deref;

use serde::Serialize;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{api::api_service::{RequestInitBuilder, METHOD_PUT}, user::secure_component::use_user_context};

#[derive(Properties, PartialEq)]
pub struct CreateActionProps {
    pub parent_id: i64
}

#[function_component(CreateAction)]
pub fn create_action(props: &CreateActionProps) -> Html {
    let action_list_visibility = use_state(|| false);
    let list_visibility = *action_list_visibility;
    let onclick = {
        let action_list_visibility = action_list_visibility.clone();
        Callback::from(move |_| {
            action_list_visibility.set(!list_visibility);    
        })
    };

    let on_action_finish = {
        let action_list_visibility = action_list_visibility.clone();
        Callback::from(move |_| {
            action_list_visibility.set(false)
        })
    };

    html! {
        <>
            <button class="file-action-create" {onclick}>
                <span class="icon-filled">{"note_add"}</span>
                <span>{"Create"}</span>
                <span class="icon-filled">{"arrow_drop_down"}</span>
            </button>
            if list_visibility {
                <CreateActionList parent_id={props.parent_id} {on_action_finish} />
            }
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct CreateActionListProps {
    pub parent_id: i64,
    pub on_action_finish: Callback<()>
}

#[function_component(CreateActionList)]
pub fn create_action_list(props: &CreateActionListProps) -> Html {
    html! {
        <ul class="file-action-create-list">
            <li class="file-action-create-list-item file-action-create-list-item--end-group">
                <NewFolderAction parent_id={props.parent_id} on_finish={props.on_action_finish.clone()} />
            </li>
            <li class="file-action-create-list-item file-action-create-list-item--start-group">
                <a>
                    <span class="icon-outlined">{"upload_file"}</span>
                    <span>{"Upload file"}</span>
                </a>
            </li>
            <li class="file-action-create-list-item file-action-create-list-item--end-group">
                <a>
                    <span class="icon-outlined">{"drive_folder_upload"}</span>
                    <span>{"Upload folder"}</span>
                </a>
            </li>
            <li class="file-action-create-list-item file-action-create-list-item--start-group">
                <a>
                    <span class="icon-outlined">{"description"}</span>
                    <span>{"Text file"}</span>
                </a>
            </li>
        </ul>
    }
}


#[derive(Properties, PartialEq)]
pub struct NewFolderActionProps {
    pub parent_id: i64,
    pub on_finish: Callback<()>
}

#[derive(Serialize, Clone)]
struct CreateFolderPayload {
    name: String
}

#[function_component(NewFolderAction)]
pub fn new_folder_action(props: &NewFolderActionProps) -> Html {
    let action_readonly_state = use_state(|| true);
    let input_ref = use_node_ref();
    let action_readonly = action_readonly_state.deref().clone();
    let onclick = {
        // Switch to input on click
        Callback::from(move |_| {
            if action_readonly {
                action_readonly_state.set(false);                
            }
        })
    };

    let auth = use_user_context();

    let onkeypress = {
        let token = auth.access_token.token.clone();
        let input_ref = input_ref.clone();
        let parent_id = props.parent_id;
        let on_finish = props.on_finish.clone();
        Callback::from(move |key: KeyboardEvent| {
            let input = input_ref.cast::<HtmlInputElement>().expect("Input exists");
            if key.code() == "Enter" {
                log::debug!("Folder name is {}", input.value());
                let token = token.clone();
                let on_finish = on_finish.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let url = format!("/api/files/create_folder/{}", parent_id);
                    let payload = CreateFolderPayload { name: input.value() };
                    RequestInitBuilder::<CreateFolderPayload>::new()
                        .set_method(METHOD_PUT)
                        .set_url(&url)
                        .set_access_token(&token)
                        .set_data(&payload)
                        .fetch()
                        .await;
                    on_finish.emit(());
                });
            }
        })
    };

    {
        // Focus input text after change status
        let input_ref = input_ref.clone();
        use_effect(move || {
            if !action_readonly {
                let input = input_ref.cast::<HtmlInputElement>().expect("Input exists");
                input.focus().unwrap_throw();
            }
            || {}
        });
    }

    html!{
        <a {onclick}>  
            <span class="icon-outlined">{"create_new_folder"}</span>
            if action_readonly {
                <span>{"New Folder"}</span>
            } else {
                <input type="text" placeholder="Folder name" class="new-folder-action-input" ref={input_ref} {onkeypress} />
            }
        </a>
    }
}
