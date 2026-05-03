pub mod handlers;
pub mod models;
mod repository;
mod service;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::get_user)
        .service(handlers::create_user);
}
