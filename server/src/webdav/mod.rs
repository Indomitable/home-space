use actix_web::{dev, FromRequest, HttpMessage, HttpRequest, HttpResponse, Responder, web};
use log::info;
use std::fmt::Write as FmtWrite;
use actix_web::http::header;
use actix_web::http::Method;
use crate::response::ok;

pub async fn handle(req: HttpRequest, body: web::Payload) -> actix_web::Result<impl Responder> {
    let mut r = String::new();
    writeln!(&mut r, "URI: {}", req.uri()).unwrap();
    writeln!(&mut r, "METHOD: {}", req.method()).unwrap();
    writeln!(&mut r, "HEADERS:").unwrap();
    for (name, value) in req.headers() {
        writeln!(&mut r, "{}: {}", name, value.to_str().unwrap()).unwrap();
    }

    let mut dev_body: dev::Payload = body.into_inner();
    let request = String::from_request(&req, &mut dev_body).await.unwrap();

    writeln!(&mut r, "BODY:").unwrap();
    writeln!(&mut r, "{}", request).unwrap();

    info!("{}", r);
    handle_request(req, dev_body).await
}


async fn handle_request(req: HttpRequest, body: dev::Payload) -> actix_web::Result<impl Responder> {
    match req.method() {
        &Method::OPTIONS => {
            let response = HttpResponse::Ok()
                .append_header(("Allow", "PROPFIND, HEAD, GET, OPTIONS"))
                .append_header(("DAV", "1"))
                .finish();
            Ok(response)
        },
        _ => {
            ok()
        }
    }
}