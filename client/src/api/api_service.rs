#![allow(unused)]
use std::borrow::Cow;

use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

pub enum ApiError {
    FetchError((u16, String))
}

pub const METHOD_GET: &'static str = "GET";
pub const METHOD_POST: &'static str = "POST";
pub const METHOD_PUT: &'static str = "PUT";
pub const METHOD_DELETE: &'static str = "DELETE";

pub struct RequestInitBuilder<'a, TData> 
where TData: Serialize + Clone {
    method: Cow<'static, str>,
    url: String,
    access_token: Option<String>,
    data: Option<&'a TData>
}

impl<'a, TData> RequestInitBuilder<'a, TData> 
where TData: Serialize + Clone {
    pub fn new() -> Self {
        Self {
            method: "GET".into(),
            url: String::default(),
            access_token: None,
            data: None
        }
    }

    pub fn set_method(&mut self, method: &str) -> &mut Self {
        self.method = method.to_owned().into();
        self
    }

    pub fn set_url(&mut self, url: &str) -> &mut Self {
        self.url = url.to_owned();
        self
    }

    pub fn set_access_token(&mut self, token: &str) -> &mut Self {
        self.access_token = Some(token.to_owned());
        self
    }

    pub fn set_data(&mut self, data: &'a TData) -> &mut Self {
        self.data = Some(data);
        self
    }

    pub async fn fetch(&self) -> Response {
        let mut init = RequestInit::new();
        init.method(&self.method);
        let headers = Headers::new().unwrap();
        if let Some(access_token) = &self.access_token {
            headers.append("Authorization", &format!("Bearer {}", access_token)).unwrap();            
        }
        if let Some(data) = &self.data {
            headers.append("Content-Type", "application/json").unwrap();
            let body = serde_json::to_string(data).unwrap();
            let jbody = JsValue::from_str(body.as_str());
            init.body(Some(&jbody));
        }
        init.headers(&JsValue::from(headers));
        init.mode(RequestMode::SameOrigin);
        fetch(&self.url, &init).await
    }
}

pub enum ResponseReader {
    TextReader(Response),
    JsonReader(Response)
}

pub enum ResponseReadError {
    ErrorResponseIsNotJson,
    ErrorJsonDeserialize(String),
    ErrorRead,
}

impl ResponseReader {
    pub async fn as_str(&self) -> Result<String, ResponseReadError> {
        match &self {
            Self::TextReader(response) |
            Self::JsonReader(response) => {
                let response_read_promise = response.text().map_err(|_| { ResponseReadError::ErrorRead })?;
                let body = JsFuture::from(response_read_promise).await.map_err(|_| { ResponseReadError::ErrorRead })?;
                let body_contents = body.as_string().unwrap_or_default();
                Ok(body_contents)
            }
        }
    }

    pub async fn as_obj<T>(&self) -> Result<T, ResponseReadError>
    where T: DeserializeOwned {
        match &self {
            Self::TextReader(_) => Err(ResponseReadError::ErrorResponseIsNotJson),
            Self::JsonReader(_) => {
                let contents = self.as_str().await;
                match contents {
                    Ok(contents) if contents.len() > 0 => {
                        let data = serde_json::from_str::<T>(contents.as_str()).map_err(|e| ResponseReadError::ErrorJsonDeserialize(e.to_string()))?;
                        Ok(data)
                    },
                    Ok(_) => Err(ResponseReadError::ErrorResponseIsNotJson),
                    Err(error) => Err(error)
                }
            }
        }
    }
}

impl From<Response> for ResponseReader {
    fn from(response: Response) -> Self {
        if let Ok(content_type) = response.headers().get("Content-Type") {
            if let Some(content_type) = content_type {
                if content_type.starts_with("application/json") {
                    return Self::JsonReader(response);
                }
            }
        }
        return Self::TextReader(response);
    }
}

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
    if status < 200 || status > 299 {
        return Err(ApiError::FetchError((status, body_contents)));
    }
    return Ok(body_contents);
}
