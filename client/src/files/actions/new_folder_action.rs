use std::{ops::Deref, rc::Rc};

use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use super::super::node_actions::NodeActions;


#[derive(Properties, PartialEq)]
pub struct NewFolderActionProps {
    pub parent_id: i64,
    pub node_actions: Rc<NodeActions>,
    pub on_finish: Callback<()>
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
        let parent_id = props.parent_id;
        let on_finish = props.on_finish.clone();
        let node_actions = props.node_actions.clone();
        Callback::from(move |key: KeyboardEvent| {
            let input = input_ref.cast::<HtmlInputElement>().expect("Input exists");
            if key.code() == "Enter" {
                log::debug!("Folder name is {}", input.value());
                node_actions.create_folder(parent_id, input.value());
                on_finish.emit(());
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
