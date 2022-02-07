use yew::prelude::*;
use user::login::Login;

mod api;
mod user;

#[function_component(App)]
fn app() -> Html {
    html! {
        <main>
            {"Wellcome to Home Space"}
            <Login></Login>
        </main>
    }
}

fn main() {
    console_log::init_with_level(log::Level::Debug).expect("Unable to initialize logger");
    yew::start_app::<App>();
}
