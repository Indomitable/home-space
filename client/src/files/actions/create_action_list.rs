use std::rc::Rc;

use yew::prelude::*;

use crate::utils::dispatcher_helpers::use_dispatcher;

use super::super::node_actions::NodeActions;
use super::file_system_api::is_file_api_supported;
use super::new_folder_action::NewFolderAction;
use super::upload_file_action::UploadFileAction;

#[derive(Properties, PartialEq)]
pub struct CreateActionProps {
    pub parent_id: i64,
    pub node_actions: Rc<NodeActions>
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

    let close_action_list = {
        let action_list_visibility = action_list_visibility.clone();

        let dispatcher = use_dispatcher();
        Callback::from(move |_| {
            action_list_visibility.set(false);
            dispatcher.borrow().publish("refresh-files-view".into(), ());
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
                <CreateActionList parent_id={props.parent_id} {close_action_list} node_actions={props.node_actions.clone()} />
            }
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct CreateActionListProps {
    pub parent_id: i64,
    pub close_action_list: Callback<()>,
    pub node_actions: Rc<NodeActions>
}

#[function_component(CreateActionList)]
pub fn create_action_list(props: &CreateActionListProps) -> Html {
    html! {
        <ul class="file-action-create-list">
            <li class="file-action-create-list-item file-action-create-list-item--end-group">
                <NewFolderAction parent_id={props.parent_id} on_finish={props.close_action_list.clone()} node_actions={props.node_actions.clone()} />
            </li>
            <li class="file-action-create-list-item file-action-create-list-item--start-group file-action-create-list-item--end-group">
                <UploadFileAction supports_open_dialog={is_file_api_supported()} parent_id={props.parent_id} close_action_list={props.close_action_list.clone()} />
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
