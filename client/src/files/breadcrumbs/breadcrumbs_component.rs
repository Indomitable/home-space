use yew::prelude::*;


#[function_component(BreadcumbsFileNav)]
pub fn file_actions() -> Html {
    html! {
        <ul class="breadcrumbs-nav">
            <li class="breadcrumb-item">
                <span class="icon-filled">{"home"}</span>
                <span>{"My files"}</span>
            </li>
        </ul>
    }
}
