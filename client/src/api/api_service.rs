use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter}, io::BufReader,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use serde::de::{DeserializeOwned};
use serde_json::{self, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct FetchError {
    err: JsValue,
}

impl Display for FetchError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.err, f)
    }
}
impl Error for FetchError {}

impl From<JsValue> for FetchError {
    fn from(value: JsValue) -> Self {
        Self { err: value }
    }
}


// pub trait ApiService {
//     async fn get<T>(url: &str) -> Result<T, FetchError>;
//     fn post<TResult, TData>(url: &str, data: &TData) -> Option<TResult>;
// }

pub struct ApiService {
}


impl<'a> ApiService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get<T >(url: &str) -> Result<T, FetchError> where T: DeserializeOwned {
        let mut init = RequestInit::new();
        init.method("GET");
        init.mode(RequestMode::SameOrigin);

        let request = Request::new_with_str_and_init(url, &init)?;
        let window = gloo_utils::window();
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;
    
        let body = JsFuture::from(resp.text()?).await?;
        let body_txt = body.as_string().expect("Body doesn't contain text value");
        let val = serde_json::from_str::<T>(body_txt.as_str()).expect("Response should be valid json");
        Ok(val)
    }

    pub fn post<TResult, TData>(url: &str, data: &TData) -> Option<TResult> {
        todo!()
    }
}