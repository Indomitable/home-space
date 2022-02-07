use log::{debug};
use serde::{Deserialize, Serialize};
use yew::{Component, Context, Html, html};
use crate::api::api_service::{ApiService};

#[derive(Deserialize, Serialize)]
struct User {
    user_name: String
}

pub enum LoginMessage {
    StartLogin,
    LoginResulted(User),
    LoginFailed
}

pub struct Login {
    user_name: String,
    password: String,
    error: String,
}



impl Component for Login {
    type Message = LoginMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            user_name: String::from(""),
            password: String::from(""),
            error: String::from(""),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            LoginMessage::StartLogin => {
                ctx.link().send_future(async {
                    let input = User { user_name: "Ventsislav".to_string() };
                    let user_result = ApiService::post::<User, User>("/api/test", &input).await;
                    return if let Ok(user) = user_result {
                        LoginMessage::LoginResulted(user)
                    } else {
                        LoginMessage::LoginFailed
                    }
                });
                false
            },
            LoginMessage::LoginResulted(user) => {
                debug!("{}", user.user_name);
                true
            },
            LoginMessage::LoginFailed => {
                false
            }
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        html! {
            <div class="login-dialog">
                <input type="text" value={self.user_name.clone()} />
                <input type="password" value={self.password.clone()} />
                <button class="login-button" onclick={ctx.link().callback(|_| LoginMessage::StartLogin)}>{"Login"}</button>
            </div>
        }
    }
}
