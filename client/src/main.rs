use yew::prelude::*;
use user::login::Login;

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
    yew::start_app::<App>();
}
