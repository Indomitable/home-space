use actix_files::Files;
use std::{path::Path};
use actix_web::{web, App, HttpServer};

use db::new_pool;

mod db;
mod response;
mod files;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = new_pool();
    // Wrap the pool to web::Data which uses Arc and can be shared between the threads
    let db_manager = web::Data::new(pool);
    HttpServer::new(move || {
        App::new()
            .app_data(db_manager.clone())
            .service(
                web::scope("/api")
                    .configure(files::init_routes)
            )
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
