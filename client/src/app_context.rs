use log::debug;
use serde_json::Value;
use std::{time::{SystemTime, UNIX_EPOCH, Duration}, ops::Add};
use yew::prelude::*;

#[derive(Debug, PartialEq, Clone)]
pub struct JwtToken {
    pub token: String,
    pub alg: String,
    pub issuer: String,
    pub valid_until: SystemTime,    
}

#[derive(Debug, PartialEq, Clone)]
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
                // Token is made of 3 parts splitted with point. 
                // 1. Header, 2 Payload. 3 Is signature but we don't keep the key on the client.
                let values = access_token
                    .split(".")
                    .take(2)
                    .map(|t| base64::decode(t).expect("token should be base 64 encoded"))
                    .map(|json| serde_json::from_str::<Value>(String::from_utf8(json).expect("token should use utf-8").as_str()).expect("token should be a json"))
                    .collect::<Vec<Value>>();
                let header = values.get(0).expect("Expect jwt to have header");
                let payload = values.get(1).expect("Expect jwt to have payload");
                debug!("{:?}", payload);
                let user_context = UserContext {
                    user_id: payload["user_id"].as_i64().expect("User Id is a number"),
                    user_name: payload["user_name"].as_str().expect("User Name should be a string").to_owned(),
                    access_token: JwtToken {
                        alg: header["alg"].as_str().expect("Alg should be a string").to_owned(),
                        issuer: payload["iss"].as_str().expect("Issuer should be a string").to_owned(),
                        token: access_token,
                        valid_until: UNIX_EPOCH.add(Duration::from_secs(payload["exp"].as_u64().expect("JWT should have expiration time")))
                    }
                };
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
    let context = use_reducer(|| AppContextInner {
        auth_context: AuthContext::NotAuthenticated
    });

    html! {
        <ContextProvider<AppContext> context={context}>
            {props.children.clone()}
        </ContextProvider<AppContext>>
    }
}