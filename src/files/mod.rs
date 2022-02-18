mod files_controller;
mod files_repository;

use actix_web::web;

pub fn init_routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/files")
            .service(files_controller::get_nodes)
            .service(files_controller::get_file)
            .service(files_controller::create_folder)
            .service(files_controller::upload_file)
            .service(files_controller::delete_node)
            // .service(files_controller::move_node)
            // .service(files_controller::copy_node)
    );
}
