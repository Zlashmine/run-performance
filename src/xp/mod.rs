pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handler::get_user_xp);
}
