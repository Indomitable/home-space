use yew::{Component, html};

pub enum LoginMessage {
    StartLogin,
    LoginResulted
}

pub struct Login {
    userName: String,
    password: String,
    error: String,
}

impl Component for Login {
    type Message = LoginMessage;
    type Properties = ();

    fn create(ctx: &yew::Context<Self>) -> Self {
        Self {
            userName: String::from(""),
            password: String::from(""),
            error: String::from(""),
        }
    }

    fn view(&self, ctx: &yew::Context<Self>) -> yew::Html {
        html! {
            <div class="login-dialog">
                <input type="text" value={self.userName.clone()} />
                <input type="password" value={self.password.clone()} />
                <button>{"Login"}</button>
            </div>
        }
    }
}
