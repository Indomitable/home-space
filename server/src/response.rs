#![allow(dead_code)]

use actix_web::{HttpResponse, web, Responder};
use serde::Serialize;

// Successful codes 2XX

/// 200 OK
pub fn ok() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok().into())
}

pub fn json<T>(payload: T) -> actix_web::Result<impl Responder>
    where T: Serialize {
    Ok(web::Json(payload))
}

/// 201 CREATED
pub fn created<T>(content: T) -> Result<HttpResponse, actix_web::Error> 
    where T: Serialize {
    let body = serde_json::to_string(&content).expect("Serialize error");
    Ok(HttpResponse::Created().body(body).into())
}

/// 202 ACCEPTED
pub fn accepted() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Accepted().into())
}

/// 204 NO_CONTENT
pub fn no_content() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::NoContent().into())
}

// Failed codes >= 300

/// 400 BAD_REQUEST
pub fn error_bad_request<T>() -> Result<T, actix_web::Error> {
    Err(actix_web::error::ErrorBadRequest("Bad Request."))
}

/// 401 UNAUTHORIZED
pub fn error_unauthorized<T>() -> Result<T, actix_web::Error> {
    Err(actix_web::error::ErrorUnauthorized("Unauthorized! Please login."))
}

/// 404 (NotFound)
pub fn error_not_found<T>() -> Result<T, actix_web::Error> {
    Err(actix_web::error::ErrorNotFound("Resource is not found"))
}

/// 500, INTERNAL_SERVER_ERROR
pub fn error_internal_server_error<T>() -> Result<T, actix_web::Error> {
    Err(actix_web::error::ErrorInternalServerError("Internal server error. Check logs."))
}
