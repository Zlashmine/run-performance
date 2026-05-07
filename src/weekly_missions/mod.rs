pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/users/{user_id}/weekly_missions",
        web::get().to(handler::get_weekly_missions),
    )
    .route(
        "/users/{user_id}/weekly_missions/{mission_id}/reroll",
        web::post().to(handler::reroll_mission),
    );
}
