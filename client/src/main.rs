use yew::prelude::*;
use yew_router::prelude::*;
use crate::router::{RouterContent};
use crate::app_context::{AppContextProvider};

mod api;
mod user;
mod router;
mod app_context;
mod home;

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
    console_log::init_with_level(log::Level::Debug).expect("Unable to initialize logger");
    yew::start_app::<App>();
}
