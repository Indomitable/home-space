use std::ops::Deref;

use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NewFolderActionProps {
    pub(crate) on_create_folder: Callback<String>
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

    let onkeypress = {
        let input_ref = input_ref.clone();
        let on_create_folder = props.on_create_folder.clone();
        Callback::from(move |key: KeyboardEvent| {
            let input = input_ref.cast::<HtmlInputElement>().expect("Input exists");
            if key.code() == "Enter" {
                on_create_folder.emit(input.value());
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
