use std::borrow::Cow;

use web_sys::HtmlInputElement;
use yew::{Component, Context, Html, html, NodeRef};
use yew_router::prelude::*;

use home_space_contracts::user::{ LoginRequest, LoginResponse };

use crate::api::api_service::{RequestInitBuilder, METHOD_POST, ResponseReader};
use crate::router::AppRoute;
use crate::app_context::AppContextAction;
use crate::utils::context_helpers::get_app_context;

pub enum LoginMessage {
    StartLogin(String, String),
    LoginResulted(LoginResponse),
    LoginFailed,
    Register
}

pub struct Login {
    user_name: String,
    password: String,
    error: Cow<'static, str>,

    user_name_ref: NodeRef,
    password_ref: NodeRef,

    is_logging: bool
}

impl Component for Login {
    type Message = LoginMessage;
    type Properties = ();

    fn create(_c: &Context<Self>) -> Self {
        Self {
            user_name: String::from(""),
            password: String::from(""),
            error: "".into(),
            user_name_ref: NodeRef::default(),
            password_ref: NodeRef::default(),
            is_logging: false
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            LoginMessage::StartLogin(user_name, password) => {                
                if !self.is_logging {
                    self.is_logging = true;
                    self.user_name = user_name.clone();
                    self.password = password.clone();
                    ctx.link().send_future(async {
                        let request = LoginRequest {
                            user_name,
                            password
                        };
                        let reader: ResponseReader = RequestInitBuilder::<LoginRequest>::new()
                            .set_method(METHOD_POST)
                            .set_url("/api/user/login")
                            .set_data(&request)
                            .fetch()
                            .await
                            .into();

                        let user_result = reader.as_obj::<LoginResponse>().await;
                        return if let Ok(user) = user_result {
                            LoginMessage::LoginResulted(user)
                        } else {
                            LoginMessage::LoginFailed
                        }
                    });
                }
                true
            },
            LoginMessage::LoginResulted(user) => {
                let app_context = get_app_context(&ctx);
                app_context.dispatch(AppContextAction::Authenticate(user.access_token));
                let navigator = ctx.link().navigator().expect("Should Have Navigator");
                navigator.push(&AppRoute::FileList{parent_id: 0});
                true
            },
            LoginMessage::LoginFailed => {
                self.error = "Unable to login! Please check you user name or password.".into();
                self.is_logging = false;
                self.password = String::default();
                true
            },
            LoginMessage::Register => {
                let navigator = ctx.link().navigator().expect("Should Have Navigator");
                navigator.push(&AppRoute::Register);
                false
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
            LoginMessage::StartLogin(user_name.value(), password.value())
        });

        html! {
            <div class="login-dialog">
                <input class="input" type="text" value={self.user_name.clone()} ref={self.user_name_ref.clone()} />
                <input class="input" type="password" value={self.password.clone()} ref={self.password_ref.clone()} />
                if self.error.len() > 0 {
                    <span>{self.error.to_owned()}</span>
                }
                <div class="login-actions">
                    <button class="button login-button" {onclick} disabled={self.is_logging}>{"Login"}</button>
                    <button class="button register-button" onclick={ctx.link().callback(|_| LoginMessage::Register)}>{"Register"}</button>
                </div>
            </div>
        }
    }
}
