
use log::debug;
use yew::{html, Html, function_component, use_context, Context, Component};
use yew_router::prelude::*;

use crate::{user::login::Login, app_context::AppContext};

#[derive(Clone, Routable, PartialEq)]
pub enum AppRoute {
    #[at("/")]
    Home,
    #[at("/secure")]
    Secure,
    #[at("/login")]
    Login,
    #[at("/register")]
    Register
}

// pub struct RouterContent {

// }

// impl RouterContent {
//     fn app_route_switch(&self, ctx: &Context<Self>, routes: &AppRoute) -> Html {
//         debug!("{:?}", context);
//         if AppRoute::Login != *routes && !context.auth_context.is_authencitated {
//             return html!( <Redirect<AppRoute> to={AppRoute::Login} /> );
//         }

//         match routes {
//             AppRoute::Home => todo!(),
//             AppRoute::Secure => todo!(),
//             AppRoute::Login => html!( <Login></Login> ),
//             AppRoute::Register => todo!(),
//         }
//     }
// }

// impl Component for RouterContent {
//     type Message = ();
//     type Properties = ();

//     fn create(ctx: &Context<Self>) -> Self {
//         Self {
//         }
//     }

//     fn view(&self, ctx: &Context<Self>) -> Html {
//         html!{
//             <Switch<AppRoute> render={Switch::render(self.app_route_switch)} />
//         }
//     }
    

// }



#[function_component(RouterContent)]
pub fn router_content() -> Html {
    let context = use_context::<AppContext>().expect("Should have context");

    html! {
        <Switch<AppRoute> render={Switch::render(move |route| { app_route_switch(&context, route) })} />
    }
}

pub fn app_route_switch(context: &AppContext, routes: &AppRoute) -> Html {   
    debug!("{:?}", context);
    if AppRoute::Login != *routes &&
        AppRoute::Register != *routes && 
        !context.auth_context.is_authencitated {
        return html!( <Redirect<AppRoute> to={AppRoute::Login} /> );
    }

    match routes {
        AppRoute::Home => todo!(),
        AppRoute::Secure => todo!(),
        AppRoute::Login => html!( <Login></Login> ),
        AppRoute::Register => todo!(),
    }
}