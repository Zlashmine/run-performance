pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/users/{user_id}/monthly_missions",
        web::get().to(handler::get_monthly_missions),
    )
    .route(
        "/users/{user_id}/monthly_missions/{mission_id}/reroll",
        web::post().to(handler::reroll_monthly_mission),
    );
}
