pub mod files_controller;
pub mod files_repository;

use actix_web::web;

pub fn init_routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/files")
            .service(files_controller::get_top_nodes)
            .service(files_controller::get_nodes)
    );
}
