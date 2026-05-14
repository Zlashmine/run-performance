pub mod handler;
pub mod models;
pub mod requirement_type;
mod repository;
pub mod service;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/users/{user_id}/goals", web::get().to(handler::list_goals))
        .route("/users/{user_id}/goals", web::post().to(handler::create_goal))
        .route("/users/{user_id}/goals/{goal_id}", web::delete().to(handler::delete_goal));
}
