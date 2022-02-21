use yew::prelude::*;

#[function_component(CreateAction)]
pub fn create_action() -> Html {
    html! {
        <button class="file-action-create">
            <span class="icon">{"note_add"}</span>
            <span>{"Create"}</span>
            <span class="icon">{"arrow_drop_down"}</span>
        </button>
    }
}
