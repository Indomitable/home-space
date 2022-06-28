
use yew::{html, Html, function_component, use_context, Callback};

use yew_router::prelude::*;

use crate::{user::login_component::Login, user::register_component::RegisterComponent, app_context::AppContext};
use crate::home::Layout;
use crate::user::secure_component::Secure;
use crate::files::files_view_component::FilesView;
use crate::favorites::favorites_component::Favorites;
use crate::recent::recent_component::Recent;
use crate::shared::shared_component::Shared;
use crate::trash::trash_component::Trash;


#[derive(Clone, Routable, PartialEq)]
pub enum AppRoute {
    #[at("/")]
    Home,
    #[at("/files/:parent_id")]
    FileList { parent_id: i64 },
    #[at("/favorites")]
    Favorites,
    #[at("/recent")]
    Recent,
    #[at("/shared")]
    Shared,
    #[at("/trash")]
    Trash,
    #[at("/login")]
    Login,
    #[at("/register")]
    Register
}

#[function_component(RouterContent)]
pub fn router_content() -> Html {
    let context = use_context::<AppContext>().expect("Should have context");

    let switch = Callback::from(move |route| { app_route_switch(&context, route) });

    html! {
        <Switch<AppRoute> render={switch} />
    }
}

pub fn app_route_switch(_context: &AppContext, routes: AppRoute) -> Html {   
    match routes {
        AppRoute::Home => html!( <Redirect<AppRoute> to={AppRoute::FileList{parent_id: 0}} /> ),
        AppRoute::FileList { parent_id} => html!{
            <Secure>
                <Layout>
                    <FilesView parent_id={parent_id} />
                </Layout>
            </Secure>
        },
        AppRoute::Favorites => html!{
            <Secure><Layout><Favorites /></Layout></Secure>
        },
        AppRoute::Recent => html!{
            <Secure><Layout><Recent /></Layout></Secure>
        },
        AppRoute::Shared => html!{
            <Secure><Layout><Shared /></Layout></Secure>
        },
        AppRoute::Trash => html!{
            <Secure><Layout><Trash /></Layout></Secure>
        },
        AppRoute::Login => html!( <Login></Login> ),
        AppRoute::Register => html!( <RegisterComponent></RegisterComponent> )
    }
}