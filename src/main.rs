use actix_files::Files;
//use r2d2_postgres::{PostgresConnectionManager, postgres::{NoTls, Config}, r2d2::ManageConnection};
use deadpool_postgres::Pool;
use serde::Serialize;
use std::{path::Path};
use actix_web::{get, web, App, HttpServer, HttpRequest, Result, Responder};

// async fn index(req: HttpRequest) -> impl Responder {
//     let content = read_to_string(Path::new("client/dist/index.html")).expect("Index file to exist");
//     HttpResponse::Ok().content_type("text/html").body(content)
// }

async fn get_index() -> actix_files::NamedFile {
    let path = Path::new("client/dist/index.html");
    actix_files::NamedFile::open(path).unwrap()
}

use db::{query, new_pool};

mod db;

#[derive(Serialize)]
struct FileNode {
    id: i64,
    user_id: i64,
    title: String,
    parent_id: Option<i64>,
    node_type: i16,
    filesystem_path: String,
    mime_type: Option<String>
}

#[get("/api/file_nodes")]
async fn file_nodes(_req: HttpRequest, pool: web::Data<Pool>) -> Result<impl Responder> {
    if let Ok(rows) = query(pool, "select * from file_nodes fn where fn.parent_id is null and user_id = $1", &[&1i64]).await {
        let mut nodes: Vec<FileNode> = vec!();
        for row in rows {
            let node = FileNode {
                id: row.get(0),
                user_id: row.get(1),
                title: row.get(2),
                parent_id: row.get(3),
                node_type: row.get(4),
                filesystem_path: row.get(5),
                mime_type: row.get(6)
            };
            nodes.push(node);
        }
        return Ok(web::Json(nodes));
    }
    return Err(actix_web::error::ErrorServiceUnavailable("Service down"));
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = new_pool();
    // Wrap the pool to web::Data which uses Arc and can be shared between the threads
    let db_manager = web::Data::new(pool);
    HttpServer::new(move || {
        App::new()
            .app_data(db_manager.clone())
            .service(file_nodes)
            .service(Files::new("/", "client/dist").index_file("index.html"))
            .default_service(web::get().to(get_index))
    })
    .bind("127.0.0.1:7070")?
    .run()
    .await
}