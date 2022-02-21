use std::borrow::Cow;

use web_sys::HtmlInputElement;
use yew::{Component, Context, Html, html, NodeRef, Callback};
use yew_router::prelude::*;

use home_space_contracts::user::{ LoginRequest, LoginResponse };

use crate::{api::api_service::post, router::AppRoute, app_context::{AppContext, AppContextAction}};

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
                        let user_result = post::<LoginResponse, LoginRequest>("/api/user/login", &request).await;
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
                let (app_context, _)  = ctx.link().context::<AppContext>(Callback::noop()).expect("Should have App context");
                app_context.dispatch(AppContextAction::Authenticate(user.access_token));
                let navigator = ctx.link().navigator().expect("Should Have Navigator");
                navigator.push(AppRoute::FileList{parent_id: 0});
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
                navigator.push(AppRoute::Register);
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
                <input type="text" value={self.user_name.clone()} ref={self.user_name_ref.clone()} />
                <input type="password" value={self.password.clone()} ref={self.password_ref.clone()} />
                if self.error.len() > 0 {
                    <span>{self.error.to_owned()}</span>
                }
                <div class="login-actions">
                    <button class="login-button" {onclick} disabled={self.is_logging}>{"Login"}</button>
                    <button class="register-button" onclick={ctx.link().callback(|_| LoginMessage::Register)}>{"Register"}</button>
                </div>
            </div>
        }
    }
}
