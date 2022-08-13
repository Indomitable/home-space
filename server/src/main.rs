use actix_files::Files;
use openssl::ssl::{SslAcceptor, SslMethod, SslFiletype};
use std::path::Path;
use actix_web::{web, App, HttpServer};

use db::new_pool;

use crate::auth::request_validator;
use crate::config::{init_config, get_host_url, get_listen_address, ssl_private_key, ssl_chain_key, ssl_listen_address};

mod config;
mod db;
mod response;
mod auth;
mod user;
mod files;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_config();
    init_logger();
    let pool = new_pool();
    // Wrap the pool to web::Data which uses Arc and can be shared between the threads
    let db_manager = web::Data::new(pool);

    log::info!("Listen on: {}", get_host_url());
    if config::is_ssl_enabled() {
        log::info!("Listen on: https://{}", ssl_listen_address());
    }

    let builder = HttpServer::new(move || {

        let auth_middleware = actix_web_httpauth::middleware::HttpAuthentication::bearer(request_validator);

        App::new()
            .app_data(db_manager.clone())
            .service(
                web::scope("/api")
                    .configure(user::init_routes)
                    .configure(files::init_routes(auth_middleware))
            )
            .default_service(web::get().to(get_index))
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
    let path = Path::new("client/dist/index.html");
    actix_files::NamedFile::open(path).unwrap()
}

fn init_logger() {
    let stdout_appender = log4rs::append::console::ConsoleAppender::builder().build();
    let config = log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout_appender)))
        .build(log4rs::config::Root::builder().appender("stdout").build(log::LevelFilter::Debug))
        .unwrap();
    log4rs::init_config(config).unwrap();
}

