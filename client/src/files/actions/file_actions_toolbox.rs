use yew::prelude::*;

use super::create_action_list::CreateAction;

#[derive(Properties, PartialEq)]
pub struct FileActionsProps {
    pub parent_id: i64
}

#[function_component(FileActions)]
pub fn file_actions(props: &FileActionsProps) -> Html {
    html! {
        <ul class="file-actions">
            <li class="file-actions-create-container">
                <CreateAction parent_id={props.parent_id} />
            </li>
        </ul>
    }
}
