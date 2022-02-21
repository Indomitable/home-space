use std::ops::Deref;

use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[function_component(CreateAction)]
pub fn create_action() -> Html {
    let action_list_visibility = use_state(|| false);
    let list_visibility = action_list_visibility.deref().clone();
    let onclick = Callback::from(move |_| {
        action_list_visibility.set(!list_visibility);    
    });
    html! {
        <>
            <button class="file-action-create" {onclick}>
                <span class="icon-filled">{"note_add"}</span>
                <span>{"Create"}</span>
                <span class="icon-filled">{"arrow_drop_down"}</span>
            </button>
            if list_visibility {
                <CreateActionList />
            }
        </>
    }
}

#[function_component(CreateActionList)]
pub fn create_action_list() -> Html {
    html! {
        <ul class="file-action-create-list">
            <li class="file-action-create-list-item file-action-create-list-item--end-group">
                <NewFolderAction />
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

#[function_component(NewFolderAction)]
pub fn new_folder_action() -> Html {
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
                <input type="text" placeholder="Folder name" class="new-folder-action-input" ref={input_ref} />
            }
        </a>
    }
}
