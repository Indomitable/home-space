use yew::prelude::*;

use crate::files::files_view_component::FileViewActions;
use super::file_action_create::CreateAction;

#[derive(Properties, PartialEq)]
pub struct FileActionsProps {
    pub parent_id: i64,
    pub selected_nodes: usize,
    pub action_callback: Callback<FileViewActions>
}

#[function_component(FileActions)]
pub fn file_actions(props: &FileActionsProps) -> Html {
    let len = props.selected_nodes;
    html! {
        <ul class="file-actions">
            if len == 0 {
                <li class="file-actions-create-container">
                    <CreateAction parent_id={props.parent_id} action_callback={props.action_callback.clone()} />
                </li>
            }
            if len > 0 {
                <li>
                    <button class="icon-button ghost-button">
                        <span class="icon-outlined">{"file_download"}</span>
                        {"Download"}
                    </button>
                </li>
                <li>
                    <button class="icon-button ghost-button">
                        <span class="icon-outlined">{"delete"}</span>
                        {"Delete"}
                    </button>
                </li>
                <li>
                    <button class="icon-button ghost-button">
                        <span class="icon-outlined">{"drive_file_move"}</span>
                        {"Move to"}
                    </button>
                </li>
                <li>                
                    <button class="icon-button ghost-button">
                        <span class="icon-outlined">{"file_copy"}</span>
                        {"Copy to"}
                    </button>
                </li>
            }
            if len == 1 {
                <li>
                    <button class="icon-button ghost-button">
                        <span class="icon-outlined">{"drive_file_rename_outline"}</span>
                        {"Rename"}
                    </button>
                </li>
            }
        </ul>
    }
}
