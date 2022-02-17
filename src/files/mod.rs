pub mod files_controller;
pub mod files_repository;

use actix_web::{web, guard, http};

pub fn init_routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/files")
            .service(files_controller::get_top_nodes)
            .service(files_controller::get_nodes)
            .service(web::resource("/upload_file")
                .guard(guard::Method(http::Method::PUT))
                .to(files_controller::upload_file)
            )
            .service(web::resource("/upload_file/{parent_id}")
                .guard(guard::Method(http::Method::PUT))
                .to(files_controller::upload_file)
            )
    );
}
