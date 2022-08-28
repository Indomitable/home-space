use actix_web::web;

pub(crate) mod paths_manager;
pub(crate) mod file_system;
pub(crate) mod files_repository;

mod files_controller;
mod favorites_controller;
pub(crate) mod db;
pub(crate) mod search;
pub(crate) mod copy_service;
pub(crate) mod version_service;
pub(crate) mod service_result;
pub(crate) mod node_create_service;
pub(crate) mod node_provide_service;
pub(crate) mod favorites_service;

pub fn init_routes<T, F>(auth_middleware: actix_web_httpauth::middleware::HttpAuthentication<actix_web_httpauth::extractors::bearer::BearerAuth, T>) -> impl FnOnce(&mut web::ServiceConfig) -> ()
where T: Fn(actix_web::dev::ServiceRequest, actix_web_httpauth::extractors::bearer::BearerAuth) -> F + 'static,
      F: std::future::Future<Output = Result<actix_web::dev::ServiceRequest, (actix_web::Error, actix_web::dev::ServiceRequest)>> + 'static {
    |config: &mut web::ServiceConfig| {
        config.service(
            web::scope("/files")
                .wrap(auth_middleware)
                .service(files_controller::get_nodes)
                .route("/download", web::get().to(files_controller::get_file))
                .service(files_controller::create_folder)
                .service(files_controller::upload_file)
                // .service(files_controller::delete_node)
                .service(files_controller::get_parents)
                .service(favorites_controller::favorite)

                // .service(files_controller::move_node)
                // .service(files_controller::copy_node)
        );
    }
}
