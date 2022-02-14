use yew::prelude::*;
use crate::user::secure_component::use_user_context;


#[function_component(Home)]
pub fn home() -> Html {
    // let context = use_context::<AppContext>().expect("Required context");
    // let user = if let AuthContext::Authenticated(user) = &context.auth_context { user } else { unreachable!() };
    let user = use_user_context();
    let message = format!("Wellcome {}", user.user_name);
    html! {
        <div>
            { message }
        </div>
    }
}


