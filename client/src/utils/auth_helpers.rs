use wasm_bindgen::throw_str;
use yew::prelude::*;

use crate::app_context::{AuthContext, UserContext};

use super::context_helpers::{use_app_context, get_app_context};

#[hook]
pub fn use_user_context() -> UserContext {
    let context = use_app_context();
    let user = if let AuthContext::Authenticated(user) = &context.auth_context {
        user
    } else {
        throw_str("User should be authenticated when accessing this view")
    };
    user.clone()
}

pub fn get_user_context<T>(ctx: &Context<T>) -> UserContext
where T: Component {
    let app_context = get_app_context(&ctx);
    let user = if let AuthContext::Authenticated(user) = &app_context.auth_context {
        user
    } else {
        throw_str("User should be authenticated when accessing this view")
    };
    user.clone()
}