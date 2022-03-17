use yew::prelude::*;
use yew_router::prelude::*;

use crate::router::RouterContent;
use crate::app_context::AppContextProvider;

mod api;
mod modal;
mod user;
mod files;
mod router;
mod dispatcher;
mod utils;
mod app_context;
mod home;
mod header;
mod left_nav;
mod favorites;
mod recent;
mod shared;
mod trash;


#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <AppContextProvider>
                <main>
                    <RouterContent />
                </main>
            </AppContextProvider>
        </BrowserRouter>
    }
}

fn main() {
    // let window = JsValue::from(gloo_utils::window());
    // let pub_sub = JsValue::from(PubSub::new());
    // Reflect::set(&window, &JsValue::from_str("pub_sub"), &pub_sub).unwrap();
    console_log::init_with_level(log::Level::Debug).expect("Unable to initialize logger");
    yew::start_app::<App>();
}
