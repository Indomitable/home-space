#![allow(unused)]
use std::borrow::Cow;

use serde::{Deserialize, Serialize};
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
where TData: Serialize {
    method: &'static str,
    url: Cow<'a, str>,
    access_token: Option<String>,
    data: Option<&'a TData>
}

impl<'a, TData> RequestInitBuilder<'a, TData> 
where TData: Serialize {
    pub fn new() -> Self {
        Self {
            method: "GET".into(),
            url: "".into(),
            access_token: None,
            data: None
        }
    }

    pub fn set_method(&mut self, method: &'static str) -> &mut Self {
        self.method = method;
        self
    }

    pub fn set_url<TUrl>(&mut self, url: TUrl) -> &mut Self
    where TUrl: Into<Cow<'a, str>> {
        self.url = url.into();
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
    JsonReader(Response),
    ErrorReader(u16)
}

pub enum FetchError {
    ErrorCode(u16),
    ErrorResponseIsNotJson,
    ErrorJsonDeserialize(String),
    ErrorRead,
}

impl ResponseReader {
    pub async fn as_str(&self) -> Result<String, FetchError> {
        match &self {
            Self::TextReader(response) |
            Self::JsonReader(response) => {
                let response_read_promise = response.text().map_err(|_| { FetchError::ErrorRead })?;
                let body = JsFuture::from(response_read_promise).await.map_err(|_| { FetchError::ErrorRead })?;
                let body_contents = body.as_string().unwrap_or_default();
                Ok(body_contents)
            },
            Self::ErrorReader(status) => {
                Err(FetchError::ErrorCode(*status))
            }
        }
    }

    pub async fn as_obj<T>(&self) -> Result<T, FetchError>
    where T: for<'a> Deserialize<'a> {
        match &self {
            Self::TextReader(_) => Err(FetchError::ErrorResponseIsNotJson),
            Self::JsonReader(_) => {
                let contents = self.as_str().await;
                match contents {
                    Ok(contents) if contents.len() > 0 => {
                        let data = serde_json::from_str::<T>(contents.as_str()).map_err(|e| FetchError::ErrorJsonDeserialize(e.to_string()))?;
                        Ok(data)
                    },
                    Ok(_) => Err(FetchError::ErrorResponseIsNotJson),
                    Err(error) => Err(error)
                }
            },
            Self::ErrorReader(status) => {
                Err(FetchError::ErrorCode(*status))
            }
        }
    }
}

impl From<Response> for ResponseReader {
    fn from(response: Response) -> Self {
        let status = response.status();
        if status >= 200 && status < 300 {
            if let Ok(content_type) = response.headers().get("Content-Type") {
                if let Some(content_type) = content_type {
                    if content_type.starts_with("application/json") {
                        return Self::JsonReader(response);
                    }
                }
            }
            return Self::TextReader(response);
        } else {
            return Self::ErrorReader(status);
        }
    }
}


async fn fetch(url: &str, init: &RequestInit) -> Response {
    let request = Request::new_with_str_and_init(url, &init)
        .unwrap_throw();
    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .unwrap_throw();
    let response: Response = resp_value
        .dyn_into()
        .unwrap_throw();
    response
}
