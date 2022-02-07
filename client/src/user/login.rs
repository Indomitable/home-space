use log::{debug, error};
use serde::Deserialize;
use yew::{Component, Context, Html, html};
use crate::api::api_service::{ApiService};

#[derive(Deserialize)]
struct User {
    userName: String
}

pub enum LoginMessage {
    StartLogin,
    LoginResulted(User)
}

pub struct Login {
    userName: String,
    password: String,
    error: String,
}



impl Component for Login {
    type Message = LoginMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            userName: String::from(""),
            password: String::from(""),
            error: String::from(""),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            LoginMessage::StartLogin => {
                ctx.link().send_future(async {
                    match ApiService::get::<User>("/api/test").await {
                        Ok(user) => LoginMessage::LoginResulted(user),
                        Err(e) => {
                            error!("{:?}", e);
                            panic!();
                        },
                    }
                });
                false
            },
            LoginMessage::LoginResulted(user) => {
                debug!("{}", user.userName);
                true
            }
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        html! {
            <div class="login-dialog">
                <input type="text" value={self.userName.clone()} />
                <input type="password" value={self.password.clone()} />
                <button class="login-button" onclick={ctx.link().callback(|_| LoginMessage::StartLogin)}>{"Login"}</button>
            </div>
        }
    }
}
