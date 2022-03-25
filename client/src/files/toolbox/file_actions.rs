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
                    <button>{"Share"}</button>
                </li>
                <li>
                    <button>{"Download"}</button>
                </li>
                <li>
                    <button>{"Delete"}</button>
                </li>
                <li>
                    <button>{"Move to"}</button>
                </li>
                <li>
                    <button>{"Copy to"}</button>
                </li>
            }
            if len == 1 {
                <li>
                    <button>{"Rename"}</button>
                </li>
            }
        </ul>
    }
}
