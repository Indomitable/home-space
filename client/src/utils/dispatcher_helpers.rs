use std::{borrow::Cow, rc::Rc, cell::RefCell};

use serde::de::DeserializeOwned;
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;


use crate::{app_context::AppContext, dispatcher::{Subscriber, Dispatcher}};

pub fn subscribe<'a, TComponent, TPayload>(ctx: &Context<TComponent>, event: Cow<'a, str>, payload_handler: Callback<TPayload>) -> Rc<Subscriber>
where TComponent: Component,
      TPayload: DeserializeOwned + 'static  {
    let (app_context, _) = ctx.link().context::<AppContext>(Callback::noop()).unwrap_throw();
    let subscriber = app_context.pub_sub.borrow_mut().subscribe(event, payload_handler); 
    subscriber
}

pub fn unsubscribe<T>(ctx: &Context<T>, subsciber: &Rc<Subscriber>) where T: Component {
    let (app_context, _) = ctx.link().context::<AppContext>(Callback::noop()).unwrap_throw();
    app_context.pub_sub.borrow_mut().unsubscribe(subsciber);
}

#[hook]
pub fn use_dispatcher() -> Rc<RefCell<Dispatcher>> {
    let context = use_context::<AppContext>().expect("Required context");
    let dispatcher = Rc::clone(&context.pub_sub);
    dispatcher
}