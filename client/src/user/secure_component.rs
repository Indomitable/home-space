use crate::{
    app_context::{AppContext, AuthContext},
    router::AppRoute,
};
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, Debug, PartialEq)]
pub struct SecureComponentProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(Secure)]
pub fn secure_component(props: &SecureComponentProps) -> Html {
    let context = use_context::<AppContext>().expect("Required context");
    match &context.auth_context {
        AuthContext::NotAuthenticated => {
            html!( <Redirect<AppRoute> to={AppRoute::Login} /> )
        }
        AuthContext::Authenticated(_) => {
            html! {
                <>
                    {props.children.clone()}
                </>
            }
        }
    }
}


