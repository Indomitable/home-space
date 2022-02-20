use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::JsValue;
use std::borrow::Cow;
use yew::prelude::*;
use js_sys::Date;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct JwtToken {
    pub token: String,
    pub valid_until: u64,    
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct UserContext {
    pub user_id: i64,
    pub user_name: String,
    pub access_token: JwtToken
}

#[derive(Debug, PartialEq, Clone)]
pub enum AuthContext {
    NotAuthenticated,
    Authenticated(UserContext)
}


#[derive(Debug, PartialEq, Clone)]
pub struct AppContextInner {
    pub auth_context: AuthContext
}

pub enum AppContextAction {
    Authenticate(String),
    LogOut
}

impl Reducible for AppContextInner {
    type Action = AppContextAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            AppContextAction::Authenticate(access_token) => {
                let token: Cow<str> = access_token.into();
                let payload = read_access_token(&token);
                debug!("{:?}", payload);
                let user_context = UserContext {
                    user_id: payload["user_id"].as_i64().expect("User Id is a number"),
                    user_name: payload["user_name"].as_str().expect("User Name should be a string").to_owned(),
                    access_token: JwtToken {
                        token: token.into_owned(),
                        valid_until: payload["exp"].as_u64().expect("JWT should have expiration time")
                    }
                };
                let _ = save_user_context_from_storage(&user_context);
                AppContextInner {
                    auth_context: AuthContext::Authenticated(user_context)
                }.into()
            },
            AppContextAction::LogOut => {
                AppContextInner {
                    auth_context: AuthContext::NotAuthenticated
                }.into()
            },
        }
    }
}

pub type AppContext = UseReducerHandle<AppContextInner>;

#[derive(Properties, Debug, PartialEq)]
pub struct AppContextProviderProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(AppContextProvider)]
pub fn app_context_provider(props: &AppContextProviderProps) -> Html {
    let auth_context = read_user_context_from_storage().map_or(AuthContext::NotAuthenticated, |user_context| { AuthContext::Authenticated(user_context) });
    let context = use_reducer(|| AppContextInner {
        auth_context
    });

    html! {
        <ContextProvider<AppContext> context={context}>
            {props.children.clone()}
        </ContextProvider<AppContext>>
    }
}


fn read_access_token(token: &str) -> Value {
    // Token is made of 3 parts splitted with point. 
    // 1. Header, 2 Payload. 3 Is signature but we don't keep the key on the client.
    let values = token
        .split(".")
        .take(2)
        .map(|t| base64::decode(t).expect("token should be base 64 encoded"))
        .map(|json| serde_json::from_str::<Value>(String::from_utf8(json).expect("token should use utf-8").as_str()).expect("token should be a json"))
        .collect::<Vec<Value>>();
    //let header = values.get(0).expect("Expect jwt to have header");
    values.get(1).expect("Expect jwt to have payload").clone()
}

const USER_CONTEXT_KEY: &str = "app_user_context_key";

fn read_user_context_from_storage() -> Result<UserContext, JsValue> {
    let context_value = read_session_storage_value(USER_CONTEXT_KEY)?;
    if context_value.len() > 0 {
        let user_context = serde_json::from_str::<UserContext>(&context_value).map_err(|_e| JsValue::from_str("Wrong value"))?;
        let time = Date::now() / 1000f64; 
        return if (user_context.access_token.valid_until as f64) < time {
            Err("Expired token".into())
        } else {
            Ok(user_context)
        }
    }
    Err("No Value".into())
}

fn save_user_context_from_storage(user_context: &UserContext) -> Result<(), JsValue> {
    let value = serde_json::to_string(user_context).map_err(|_e| JsValue::from_str("Wrong value"))?; 
    save_session_storage_value(USER_CONTEXT_KEY, &value)
}

fn read_session_storage_value(key: &str) -> Result<String, JsValue> {
    if let Some(storage) = gloo_utils::window().session_storage()? {
        if let Some(value) = storage.get(key)? {
            return Ok(value);
        }
    }
    Err("No Value".into())
}

fn save_session_storage_value(key: &str, value: &str) -> Result<(), JsValue> {
    if let Some(storage) = gloo_utils::window().session_storage()? {
        return storage.set(key, value);
    }
    Err("No Value".into())
}
