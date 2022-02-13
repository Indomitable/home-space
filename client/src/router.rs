
use yew::{html, Html, function_component, use_context};
use crate::home::{Home};
use crate::user::secure_component::Secure;
use yew_router::prelude::*;

use crate::{user::login::Login, user::register::RegisterComponent, app_context::{AppContext, AuthContext}};

#[derive(Clone, Routable, PartialEq)]
pub enum AppRoute {
    #[at("/")]
    Home,
    #[at("/login")]
    Login,
    #[at("/register")]
    Register
}

#[function_component(RouterContent)]
pub fn router_content() -> Html {
    let context = use_context::<AppContext>().expect("Should have context");

    html! {
        <Switch<AppRoute> render={Switch::render(move |route| { app_route_switch(&context, route) })} />
    }
}

pub fn app_route_switch(context: &AppContext, routes: &AppRoute) -> Html {   

    let user_context = if let AuthContext::Authenticated(user) = &context.auth_context {
        user
    } else {
        unreachable!()
    };
    match routes {
        AppRoute::Home => html!(<Secure><Home user_context={user_context.clone()}></Home></Secure> ),
        AppRoute::Login => html!( <Login></Login> ),
        AppRoute::Register => html!( <RegisterComponent></RegisterComponent> )
    }
}