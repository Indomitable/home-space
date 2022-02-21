use yew::prelude::*;

use super::actions::create_component::CreateAction;

#[function_component(FileActions)]
pub fn file_actions() -> Html {
    html! {
        <ul>
            <CreateAction />
        </ul>
    }
}
