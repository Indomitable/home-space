use actix_cors::Cors;
use actix_files::Files;
use openssl::ssl::{SslAcceptor, SslMethod, SslFiletype};
use std::path::Path;
use actix_web::{web, App, HttpServer, middleware::Condition};

use crate::auth::request_validator;
use crate::config::{init_config, get_host_url, get_listen_address, ssl_private_key, ssl_chain_key, ssl_listen_address};

mod config;
mod db;
mod response;
mod auth;
mod user;
mod files;
mod sorting;
mod ioc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_config();
    init_logger();
    let container = ioc::container::Contrainer::new();
    let container_data = web::Data::new(container);

    log::info!("Listen on: {}", get_host_url());
    if config::is_ssl_enabled() {
        log::info!("Listen on: https://{}", ssl_listen_address());
    }

    let builder = HttpServer::new(move || {

        let auth_middleware = actix_web_httpauth::middleware::HttpAuthentication::bearer(request_validator);

        let is_prod = !config::is_prod();

        let mut app = App::new()
            .app_data(container_data.clone())
            .wrap(Condition::new(is_prod,
                Cors::default()
                    .allowed_origin("http://127.0.0.1:5173")
                    .allowed_origin("http://localhost:7070")
                    .allow_any_header()
                    .allow_any_method()
                ))
            .service(
                web::scope("/api")
                    .configure(user::init_routes)
                    .configure(files::init_routes(auth_middleware))
            );

            if is_prod {
                app = app
                .service(Files::new("/", "client-js/dist")
                    .index_file("index.html")
                );                
            }
            app = app.default_service(web::get().to(get_index));
            app
    })
    .bind(get_listen_address())?;


    if config::is_ssl_enabled() {
        let mut ssl_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        ssl_builder
            .set_private_key_file(ssl_private_key(), SslFiletype::PEM)
            .unwrap();
        ssl_builder.set_certificate_chain_file(ssl_chain_key()).unwrap();
        builder
        .bind_openssl(ssl_listen_address(), ssl_builder)?
        .run().await
    } else {
        builder.run().await
    }
}

async fn get_index() -> actix_files::NamedFile {
    let path = Path::new("client-js/dist/index.html");
    actix_files::NamedFile::open(path).unwrap()
}

fn init_logger() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Utc::now().format("[%Y-%m-%dT%H:%M:%SZ]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()
        .unwrap();
}

