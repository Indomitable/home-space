use wasm_bindgen::{UnwrapThrowExt, throw_str};
use yew::prelude::*;

use crate::app_context::{AppContext, AuthContext, UserContext};

#[hook]
pub fn use_user_context() -> UserContext {
    let context = use_context::<AppContext>().expect("Required context");
    let user = if let AuthContext::Authenticated(user) = &context.auth_context {
        user
    } else {
        throw_str("User should be authenticated when accessing this view")
    };
    user.clone()
}

pub fn get_user_context<T>(ctx: &Context<T>) -> UserContext
where T: Component {
    let (app_context, ..) = ctx.link().context::<AppContext>(Callback::noop()).unwrap_throw();
    let user = if let AuthContext::Authenticated(user) = &app_context.auth_context {
        user
    } else {
        throw_str("User should be authenticated when accessing this view")
    };
    user.clone()
}