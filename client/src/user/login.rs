use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::{Component, Context, Html, html, NodeRef, Callback};
use yew_router::prelude::*;

use crate::{api::api_service::{ApiService}, router::AppRoute, app_context::{AppContext, AppContextAction}};

#[derive(Serialize)]
pub struct LoginRequest {
    user_name: String,
    password: String
}

#[derive(Deserialize)]
pub struct LoginResponse {
    // user_id: i64,
    // user_name: String,
    access_token: String,
}


pub enum LoginMessage {
    StartLogin(String, String),
    LoginResulted(LoginResponse),
    LoginFailed,
    Register
}

pub struct Login {
    user_name: String,
    password: String,
    error: String,

    user_name_ref: NodeRef,
    password_ref: NodeRef,
}

impl Component for Login {
    type Message = LoginMessage;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        Self {
            user_name: String::from(""),
            password: String::from(""),
            error: String::from(""),
            user_name_ref: NodeRef::default(),
            password_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            LoginMessage::StartLogin(user_name, password) => {
                ctx.link().send_future(async {
                    let request = LoginRequest {
                        user_name,
                        password
                    };
                    let user_result = ApiService::post::<LoginResponse, LoginRequest>("/api/auth/login", &request).await;
                    return if let Ok(user) = user_result {
                        LoginMessage::LoginResulted(user)
                    } else {
                        LoginMessage::LoginFailed
                    }
                });
                false
            },
            LoginMessage::LoginResulted(user) => {
                let (app_context, _)  = ctx.link().context::<AppContext>(Callback::noop()).expect("Should have App context");
                app_context.dispatch(AppContextAction::Authenticate(user.access_token));
                let history = ctx.link().navigator().expect("Should Have history");
                history.push(AppRoute::Home);
                true
            },
            LoginMessage::LoginFailed => {
                false
            },
            LoginMessage::Register => {
                let history = ctx.link().navigator().expect("Should Have history");
                history.push(AppRoute::Register);
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
                <div class="login-actions">
                    <button class="login-button" {onclick}>{"Login"}</button>
                    <button class="register-button" onclick={ctx.link().callback(|_| LoginMessage::Register)}>{"Register"}</button>
                </div>
            </div>
        }
    }
}
