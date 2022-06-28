use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use home_space_contracts::user::{RegisterRequest, LoginResponse};

use crate::api::api_service::{RequestInitBuilder, METHOD_POST, ResponseReader};
use crate::app_context::AppContextAction;
use crate::router::AppRoute;
use crate::utils::context_helpers::get_app_context;

pub(crate) enum RegisterMessage {
    StartRegister(String, String),
    RegisterSuccessful(LoginResponse),
    RegisterFailed(String)
}

pub(crate) struct RegisterComponent {
    user_name: String,
    password: String,
    error: String,

    user_name_ref: NodeRef,
    password_ref: NodeRef,
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

                    let reader: ResponseReader = RequestInitBuilder::<RegisterRequest>::new()
                        .set_method(METHOD_POST)
                        .set_url("/api/user/register")
                        .set_data(&request)
                        .fetch()
                        .await
                        .into();

                    let user_result = reader.as_obj::<LoginResponse>().await;
                    return match user_result {
                        Ok(res) => RegisterMessage::RegisterSuccessful(res),
                        Err(_) => RegisterMessage::RegisterFailed("Registration failed".to_owned())
                    }
                });
                callback.emit(input);
                false                
            },
            RegisterMessage::RegisterSuccessful(login_response) => {
                let app_context = get_app_context(&ctx);
                app_context.dispatch(AppContextAction::Authenticate(login_response.access_token));
                let navigator = ctx.link().navigator().expect("Should Have Navigator");
                navigator.push(&AppRoute::FileList{parent_id: 0});
                true
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
                <input class="input" type="text" value={self.user_name.clone()} ref={self.user_name_ref.clone()} />
                <input class="input" type="password" value={self.password.clone()} ref={self.password_ref.clone()} />
                if self.error.len() > 0 {
                    <span>{self.error.clone()}</span>
                }
                <div class="register-actions">
                    <button class="button register-button" {onclick}>{"Register"}</button>
                </div>
            </div>
        }
    }


}
