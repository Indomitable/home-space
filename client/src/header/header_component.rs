use yew::prelude::*;

use crate::user::secure_component::use_user_context;

#[function_component(Header)]
pub fn header() -> Html {
    let user = use_user_context();
    html!{
        <header class="top-header">
            { "Wellcome: " } {user.user_name}
        </header>
    }
}
