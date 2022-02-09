use std::error::Error;
use std::fmt::Display;

use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

#[derive(Debug)]
pub struct ApiError {
    pub error: String,
    pub code: u16,
}

impl Error for ApiError {}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status Code: {}", self.code)
    }
}

pub struct ApiService {}

impl<'a> ApiService {
    pub async fn get<TResult>(url: &str) -> Result<TResult, ApiError>
    where
        TResult: DeserializeOwned,
    {
        let mut init = RequestInit::new();
        init.method("GET");
        init.mode(RequestMode::SameOrigin);

        let response = fetch(url, &init).await;
        read_body(response).await.map(|contents| {
            serde_json::from_str::<TResult>(contents.as_str())
                .expect("Response should be a valid json")
        })
    }

    pub async fn post<TResult, TData>(url: &str, data: &TData) -> Result<TResult, ApiError>
    where
        TResult: DeserializeOwned,
        TData: Serialize,
    {
        let init = request_init(data);
        let response = fetch(url, &init).await;

        read_body(response).await.map(|contents| {
            serde_json::from_str::<TResult>(contents.as_str())
                .expect("Response should be a valid json")
        })
    }

    /// Executes post request which doesn't return data if Ok is returned then request is successful
    pub async fn post_no_result<TData>(url: &str, data: &TData) -> Result<(), ApiError>
    where
        TData: Serialize,
    {
        let init = request_init(data);
        let response = fetch(url, &init).await;

        read_body(response).await.map(|_| ())
    }
}

fn request_init<TData>(data: &TData) -> RequestInit
where
    TData: Serialize,
{
    let mut init = RequestInit::new();
    init.method("POST");
    let headers = Headers::new().expect("Unable to create headers");
    headers
        .append("Content-Type", "application/json")
        .expect("Should add content type");
    init.headers(&JsValue::from(headers));
    let body = serde_json::to_string(data).expect("Data should be serializable");
    let user = JsValue::from_str(body.as_str());
    init.body(Some(&user));
    //let data = serde_json::to_string(data).expect("Data should be serializable");
    init.mode(RequestMode::SameOrigin);
    init
}

async fn fetch(url: &str, init: &RequestInit) -> Response {
    let request = Request::new_with_str_and_init(url, &init)
        .expect("Unable to crate request from request init");
    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .expect("Error while execution the request");
    let response: Response = resp_value
        .dyn_into()
        .expect("Unable to convert JValue to Responce");
    response
}

async fn read_body(response: Response) -> Result<String, ApiError> {
    let status = response.status();
    let body = JsFuture::from(response.text().expect("Unable to read response body"))
        .await
        .expect("Error while getting body");
    let body_contents = body.as_string().expect("Body doesn't contain text value");
    if status != 200 {
        return Err(ApiError {
            code: status,
            error: body_contents,
        });
    }
    return Ok(body_contents);
}
