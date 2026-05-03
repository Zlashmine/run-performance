pub mod handlers;
pub mod models;
pub mod parser;
mod repository;
mod service;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::get_activities)
        .service(handlers::get_activity_detail)
        .service(handlers::get_trackpoints)
        .service(handlers::upload_files);
}
