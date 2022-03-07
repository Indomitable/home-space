use yew::prelude::*;

use super::new_folder_component::NewFolderAction;
use super::upload_file_component::UploadFileAction;

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

    let close_action_list = {
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
                <CreateActionList parent_id={props.parent_id} {close_action_list} />
            }
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct CreateActionListProps {
    pub parent_id: i64,
    pub close_action_list: Callback<()>
}

#[function_component(CreateActionList)]
pub fn create_action_list(props: &CreateActionListProps) -> Html {
    html! {
        <ul class="file-action-create-list">
            <li class="file-action-create-list-item file-action-create-list-item--end-group">
                <NewFolderAction parent_id={props.parent_id} on_finish={props.close_action_list.clone()} />
            </li>
            <li class="file-action-create-list-item file-action-create-list-item--start-group">
                <UploadFileAction onclick={props.close_action_list.clone()} />
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

