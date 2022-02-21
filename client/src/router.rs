
use yew::{html, Html, function_component, use_context};

use yew_router::prelude::*;

use crate::{user::login_component::Login, user::register_component::RegisterComponent, app_context::AppContext};
use crate::home::Layout;
use crate::user::secure_component::Secure;
use crate::files::files_view_component::FilesView;

#[derive(Clone, Routable, PartialEq)]
pub enum AppRoute {
    #[at("/")]
    Home,
    #[at("/files/:parent_id")]
    FileList { parent_id: i64 },
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

pub fn app_route_switch(_context: &AppContext, routes: &AppRoute) -> Html {   
    match routes {
        AppRoute::Home => html!( <Redirect<AppRoute> to={AppRoute::FileList{parent_id: 0}} /> ),
        AppRoute::FileList { parent_id} => html!{
            <Secure>
                <Layout>
                    <FilesView parent_id={parent_id} />
                </Layout>
            </Secure>
        },
        AppRoute::Login => html!( <Login></Login> ),
        AppRoute::Register => html!( <RegisterComponent></RegisterComponent> )
    }
}