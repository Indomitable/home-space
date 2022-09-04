use actix_web::web;

mod notifications_socket;

pub fn init_routes(config: &mut web::ServiceConfig) {
    config
        .route("/notifications", web::get().to(notifications_socket::index));
}
