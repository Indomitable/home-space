#![allow(dead_code)]

use actix_web::HttpResponse;

// Successful codes 2XX

/// 200 OK
pub fn ok() -> Result<HttpResponse, actix_web::Error> {
    return Ok(HttpResponse::Ok().into());
}

/// 201 CREATED
pub fn created() -> Result<HttpResponse, actix_web::Error> {
    return Ok(HttpResponse::Created().into());
}

/// 202 ACCEPTED
pub fn accepted() -> Result<HttpResponse, actix_web::Error> {
    return Ok(HttpResponse::Accepted().into());
}

/// 204 NO_CONTENT
pub fn no_content() -> Result<HttpResponse, actix_web::Error> {
    return Ok(HttpResponse::NoContent().into());
}

// Failed codes >= 300

/// 400 BAD_REQUEST
pub fn error_bad_request<T>() -> Result<T, actix_web::Error> {
    return Err(actix_web::error::ErrorBadRequest("Bad Request."));
}

/// 401 UNAUTHORIZED
pub fn error_unauthorized<T>() -> Result<T, actix_web::Error> {
    return Err(actix_web::error::ErrorUnauthorized("Unauthorized! Please login."));
}

/// 404 (NotFound)
pub fn error_not_found<T>() -> Result<T, actix_web::Error> {
    return Err(actix_web::error::ErrorNotFound("Resource is not found"));
}

/// 500, INTERNAL_SERVER_ERROR
pub fn error_internal_server_error<T>() -> Result<T, actix_web::Error> {
    return Err(actix_web::error::ErrorInternalServerError("Internal server error. Check logs."));
}
