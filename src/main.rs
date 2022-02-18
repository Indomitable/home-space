use actix_files::Files;
use std::{path::Path};
use actix_web::{web, App, HttpServer};

use db::new_pool;

mod db;
mod response;
mod user;
mod files;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();
    let pool = new_pool();
    // Wrap the pool to web::Data which uses Arc and can be shared between the threads
    let db_manager = web::Data::new(pool);
    log::info!("Listen on: http://127.0.0.1:7070");
    HttpServer::new(move || {
        App::new()
            .app_data(db_manager.clone())
            .service(
                web::scope("/api")
                    .configure(files::init_routes)
            )
            .configure(user::init_routes)
            .configure(files::init_routes)
            .service(Files::new("/", "client/dist").index_file("index.html"))
            .default_service(web::get().to(get_index))
    })
    .bind("127.0.0.1:7070")?
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
