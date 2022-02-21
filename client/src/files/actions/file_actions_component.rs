use yew::prelude::*;

use super::create_component::CreateAction;

#[function_component(FileActions)]
pub fn file_actions() -> Html {
    html! {
        <ul class="file-actions">
            <li class="file-actions-create-container">
                <CreateAction />
            </li>
        </ul>
    }
}
