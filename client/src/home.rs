use yew::prelude::*;

use crate::app_context::UserContext;


#[derive(Properties, PartialEq)]
pub struct HomeProps {
    pub user_context: UserContext
}

// impl UserContextProps for HomeProps {
 
// }


#[function_component(Home)]
pub fn home(props: &HomeProps) -> Html {
    // let context = use_context::<AppContext>().expect("Required context");
    // let user = if let AuthContext::Authenticated(user) = &context.auth_context { user } else { unreachable!() };    
    let message = format!("Wellcome {}", props.user_context.user_name);
    html! {
        <div>
            { message }
        </div>
    }
}


