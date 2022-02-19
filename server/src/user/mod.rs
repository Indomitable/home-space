mod user_controller;
mod user_repository;
mod token;

use actix_web::web;

pub fn init_routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/user")
        .service(user_controller::login)
        .service(user_controller::register)
    );
}
