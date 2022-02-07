use std::error::Error;
use std::fmt::{Display};

use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, Headers};
use serde::{de::{DeserializeOwned}, Serialize};
use serde_json;

#[derive(Debug)]
pub struct ApiError {
    code: u16,
}

impl Error for ApiError {
    
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status Code: {}", self.code)
    }
}


pub struct ApiService {
}


impl<'a> ApiService {
    pub async fn get<TResult>(url: &str) -> Result<TResult, ApiError>
        where TResult: DeserializeOwned {
        let mut init = RequestInit::new();
        init.method("GET");
        init.mode(RequestMode::SameOrigin);

        let request = Request::new_with_str_and_init(url, &init).expect("Unable to crate request from request init");
        let window = gloo_utils::window();
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.expect("Error while execution the request");
        let resp: Response = resp_value.dyn_into().expect("Unable to convert JValue to Responce");
        if resp.status() != 200 {
            return Err(ApiError{ code: resp.status() });
        }
        let body = JsFuture::from(resp.text().expect("Unable to read response body")).await.expect("Error while getting body");
        let body_txt = body.as_string().expect("Body doesn't contain text value");
        let val = serde_json::from_str::<TResult>(body_txt.as_str()).expect("Response should be a valid json");
        Ok(val)
    }

    pub async fn post<TResult, TData>(url: &str, data: &TData) -> Result<TResult, ApiError>
        where TResult: DeserializeOwned,
              TData: Serialize {
        let mut init = RequestInit::new();
        init.method("POST");
        let headers = Headers::new().expect("Unable to create headers");
        headers.append("Content-Type", "application/json").expect("Should add content type");
        init.headers(&JsValue::from(headers));
        let body = serde_json::to_string(data).expect("Data should be serializable");
        let user = JsValue::from_str(body.as_str());
        init.body(Some(&user));
        //let data = serde_json::to_string(data).expect("Data should be serializable");
        init.mode(RequestMode::SameOrigin);

        let request = Request::new_with_str_and_init(url, &init).expect("Unable to crate request from request init");
        let window = gloo_utils::window();
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.expect("Error while execution the request");
        let resp: Response = resp_value.dyn_into().expect("Unable to convert JValue to Responce");
        if resp.status() != 200 {
            return Err(ApiError{ code: resp.status() });
        }
        let body = JsFuture::from(resp.text().expect("Unable to read response body")).await.expect("Error while getting body");
        let body_txt = body.as_string().expect("Body doesn't contain text value");
        let val = serde_json::from_str::<TResult>(body_txt.as_str()).expect("Response should be a valid json");
        Ok(val)
    }
}