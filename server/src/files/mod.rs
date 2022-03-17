use actix_web::web;

mod file_system;
mod files_controller;
mod files_repository;
mod favorites_controller;

pub fn init_routes<T, F>(auth_middleware: actix_web_httpauth::middleware::HttpAuthentication<actix_web_httpauth::extractors::bearer::BearerAuth, T>) -> impl FnOnce(&mut web::ServiceConfig) -> ()
where T: Fn(actix_web::dev::ServiceRequest, actix_web_httpauth::extractors::bearer::BearerAuth) -> F + 'static,
      F: std::future::Future<Output = Result<actix_web::dev::ServiceRequest, actix_web::Error>> + 'static {
    return |config: &mut web::ServiceConfig| {
        config.service(
            web::scope("/files")
                .wrap(auth_middleware)
                .service(files_controller::get_nodes)
                .service(files_controller::get_file)
                .service(files_controller::create_folder)
                .service(files_controller::upload_file)
                .service(files_controller::delete_node)
                .service(files_controller::get_parents)
                .service(favorites_controller::set_favorite)
                // .service(files_controller::move_node)
                // .service(files_controller::copy_node)
        );
    };
}
