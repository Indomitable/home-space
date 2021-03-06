use yew::prelude::*;

use crate::app_context::{AppContext, AppContextAction};

#[function_component(Logout)]
pub fn logout_component() -> Html {
    let context = use_context::<AppContext>().unwrap();
    let onclick = Callback::from(move |_| {
        context.dispatch(AppContextAction::LogOut);
    });

    html! {
        <button class="button ghost-button" {onclick}>
            <span class="icon-filled">{"logout"}</span>
        </button>
    }
}
