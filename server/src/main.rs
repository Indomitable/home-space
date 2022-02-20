use actix_files::Files;
use std::{path::Path, env::{self, VarError}};
use actix_web::{web, App, HttpServer};

use db::new_pool;

use crate::auth::request_validator;

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

    let address = format!("{}:{}", env::var("SERVER_NAME").unwrap(), env::var("SERVER_PORT").unwrap());
    let http_address = format!("{}://{}", env::var("SERVER_SCHEMA").unwrap(), address);

    log::info!("Listen on: {}", http_address);
    HttpServer::new(move || {

        let auth_middleware = actix_web_httpauth::middleware::HttpAuthentication::bearer(request_validator);

        App::new()
            .app_data(db_manager.clone())
            .service(
                web::scope("/api")
                    .configure(user::init_routes)
                    .configure(files::init_routes(auth_middleware))
            )
            .service(Files::new("/", "client/dist").index_file("index.html"))
            .default_service(web::get().to(get_index))
    })
    .bind(address)?
    .run()
    .await
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

fn init_config() {
    let env = env::var("APP_ENVIRONMENT").or::<VarError>(Ok("dev".into())).unwrap();
    let config_path = format!(".env.{}", env);
    dotenv::from_filename(&config_path).unwrap();
}
