use yew::prelude::*;

use crate::files::files_view_component::FileViewActions;
use crate::utils::dispatcher_helpers::use_dispatcher;

use super::super::actions::new_folder_action::NewFolderAction;
use super::super::actions::upload::upload_file_action::UploadFileAction;

#[derive(Properties, PartialEq)]
pub struct CreateActionProps {
    pub parent_id: i64,
    pub action_callback: Callback<FileViewActions>
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

    let on_create_folder = {
        let action_list_visibility = action_list_visibility.clone();
        let action_callback = props.action_callback.clone();
        Callback::from(move |folder_name| {
            action_callback.emit(FileViewActions::FileNodesCreateFolder(folder_name));
            action_list_visibility.set(false);
        })
    };

    html! {
        <>
            <button class="file-action-create ghost-button" {onclick}>
                <span class="icon-outlined">{"note_add"}</span>
                <span>{"Create"}</span>
                <span class="icon-filled">{"arrow_drop_down"}</span>
            </button>
            if list_visibility {
                <CreateActionList parent_id={props.parent_id} {close_action_list} {on_create_folder} />
            }
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct CreateActionListProps {
    pub parent_id: i64,
    pub close_action_list: Callback<()>,
    pub on_create_folder: Callback<String>
}

#[function_component(CreateActionList)]
pub fn create_action_list(props: &CreateActionListProps) -> Html {
    html! {
        <ul class="file-action-create-list popup">
            <li class="file-action-create-list-item file-action-create-list-item--end-group">
                <NewFolderAction on_create_folder={props.on_create_folder.clone()}  />
            </li>
            <li class="file-action-create-list-item file-action-create-list-item--start-group file-action-create-list-item--end-group">
                <UploadFileAction parent_id={props.parent_id} close_action_list={props.close_action_list.clone()} />
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
