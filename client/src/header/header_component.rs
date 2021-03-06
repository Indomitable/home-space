use yew::prelude::*;

use crate::utils::auth_helpers::use_user_context;
use super::logout_component::Logout;

#[function_component(Header)]
pub fn header() -> Html {
    let user = use_user_context();
    html!{
        <header class="top-header">
            { "Wellcome: " } {user.user_name}

            <Logout />
        </header>
    }
}
