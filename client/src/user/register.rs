use serde::Serialize;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::api::api_service::ApiService;

// enum RegisterAction {
//     StartRegister,
//     RegisterSuccessful,
//     RegisterFailed(String)
// }

// struct RegisterState {
//     user_name: String,
//     password: String,
//     error: String
// }

// #[derive(Serialize)]
// struct RegisterRequest {
//     user_name: String,
//     password: String,
// }

// impl Default for RegisterState {
//     fn default() -> Self {
//         Self { user_name: Default::default(), password: Default::default(), error: Default::default() }
//     }
// }

// impl Reducible for RegisterState {
//     type Action = RegisterAction;

//     fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
//         match action {
//             RegisterAction::StartRegister => {
//                 let input = RegisterRequest {
//                     user_name: self.user_name.clone(), 
//                     password: self.password.clone()
//                 };
//                 async {
//                     let user_result = ApiService::post_no_result::<RegisterRequest>("/api/auth/register", &input).await;
                    
//                 };
//             },
//             RegisterAction::RegisterSuccessful => todo!(),
//             RegisterAction::RegisterFailed(_) => todo!(),
//         }
//         self
//     }
// }


// #[function_component(Register)]
// fn register() -> Html {
//     let state = use_reducer(RegisterState::default);
//     let register = {
//         let state = state.clone();
//         Callback::from(move |_| state.dispatch(RegisterAction::StartRegister))
//     };

//     html! {
//         <div class="register-dialog">
//             <input type="text" value={state.user_name.clone()} />
//             <input type="password" value={state.password.clone()} />
//             <div class="register-actions">
//                 <button class="register-button" onclick={register}>{"Register"}</button>
//             </div>
//         </div>
//     }
// }


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
                    let user_result = ApiService::post_no_result::<RegisterRequest>("/api/auth/register", &request).await;
                    return if let Err(error) = user_result {
                        RegisterMessage::RegisterFailed(error.error)
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
