use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use crate::app_context::AppContext;

#[hook]
pub fn use_app_context() -> AppContext {
    let app_context = use_context::<AppContext>().unwrap_throw();
    app_context
}

pub fn get_app_context<T>(ctx: &Context<T>) -> AppContext
where T: Component {
    let (app_context, ..) = ctx.link().context::<AppContext>(Callback::noop()).unwrap_throw();
    app_context
}