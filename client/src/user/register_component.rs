use serde::Serialize;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::api::api_service::{post_no_result, ApiError};

pub(crate) enum RegisterMessage {
    StartRegister(String, String),
    RegisterSuccessful,
    RegisterFailed(String)
}

pub(crate) struct RegisterComponent {
    user_name: String,
    password: String,
    error: String,

    user_name_ref: NodeRef,
    password_ref: NodeRef,
}

#[derive(Serialize)]
struct RegisterRequest {
    user_name: String,
    password: String,
}

impl Component for RegisterComponent {
    type Message = RegisterMessage;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            user_name: String::from(""),
            password: String::from(""),
            error: String::from(""),
            user_name_ref: NodeRef::default(),
            password_ref: NodeRef::default()
        }
    }
 
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            RegisterMessage::StartRegister(user_name, password) => {
                let input = RegisterRequest {
                    user_name: user_name, 
                    password: password
                };
                let callback = ctx.link().callback_future(|request: RegisterRequest| async move {
                    let user_result = post_no_result::<RegisterRequest>("/api/user/register", &request).await;
                    return if let Err(ApiError::FetchError(error)) = user_result {
                        RegisterMessage::RegisterFailed(error.1)
                    } else {
                        RegisterMessage::RegisterSuccessful
                    }
                });
                callback.emit(input);
                false                
            },
            RegisterMessage::RegisterSuccessful => {
                gloo_utils::window().alert_with_message("Registered").unwrap();
                false
            },
            RegisterMessage::RegisterFailed(error) => {
                self.error = error;
                true
            }
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        let link = ctx.link();
        let user_name_ref_c = self.user_name_ref.clone();
        let password_ref_c = self.password_ref.clone();

        let onclick = link.callback(move |_| {
            let user_name = user_name_ref_c.cast::<HtmlInputElement>().unwrap();
            let password = password_ref_c.cast::<HtmlInputElement>().unwrap();
            RegisterMessage::StartRegister(user_name.value(), password.value())
        });

        html! {
            <div class="register-dialog">
                <input type="text" value={self.user_name.clone()} ref={self.user_name_ref.clone()} />
                <input type="password" value={self.password.clone()} ref={self.password_ref.clone()} />
                if self.error.len() > 0 {
                    <span>{self.error.clone()}</span>
                }
                <div class="register-actions">
                    <button class="register-button" {onclick}>{"Register"}</button>
                </div>
            </div>
        }
    }


}
