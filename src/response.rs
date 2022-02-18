#![allow(dead_code)]

use actix_web::HttpResponse;

pub fn ok() -> Result<HttpResponse, actix_web::Error> {
    return Ok(HttpResponse::Ok().into());
}

pub fn created() -> Result<HttpResponse, actix_web::Error> {
    return Ok(HttpResponse::Created().into());
}

pub fn no_content() -> Result<HttpResponse, actix_web::Error> {
    return Ok(HttpResponse::NoContent().into());
}

pub fn not_found() -> Result<HttpResponse, actix_web::Error> {
    return Ok(HttpResponse::NotFound().into());
}

pub fn error_server_unavailable<T>() -> Result<T, actix_web::Error> {
    return Err(actix_web::error::ErrorServiceUnavailable("Service down"));
}

pub fn error_unauthorized<T>() -> Result<T, actix_web::Error> {
    return Err(actix_web::error::ErrorUnauthorized("Unauthorized! Please login."));
}

pub fn error_bad_request<T>() -> Result<T, actix_web::Error> {
    return Err(actix_web::error::ErrorBadRequest("Bad Request."));
}
