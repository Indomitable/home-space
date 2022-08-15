use wasm_bindgen::throw_str;
use yew::prelude::*;

use crate::app_context::{AuthContext, UserContext, JwtToken};

use super::context_helpers::{use_app_context, get_app_context};

#[hook]
pub fn use_user_context() -> UserContext {
    return UserContext {
        user_id: 0,
        user_name: "test".to_string(),
        access_token: JwtToken {
            token: "".to_string(),
            valid_until: 10
        }
    };
    // let context = use_app_context();
    // let user = if let AuthContext::Authenticated(user) = &context.auth_context {
    //     user
    // } else {
    //     throw_str("User should be authenticated when accessing this view")
    // };
    // user.clone()
}

pub fn get_user_context<T>(ctx: &Context<T>) -> UserContext
where T: Component {
    return UserContext {
        user_id: 0,
        user_name: "test".to_string(),
        access_token: JwtToken {
            token: "".to_string(),
            valid_until: 10
        }
    };
    // let app_context = get_app_context(&ctx);
    // let user = if let AuthContext::Authenticated(user) = &app_context.auth_context {
    //     user
    // } else {
    //     throw_str("User should be authenticated when accessing this view")
    // };
    // user.clone()
}