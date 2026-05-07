pub mod handlers;
pub mod models;
pub mod progression;
pub mod requirement_type;
pub use requirement_type::RequirementType;
pub mod status;
pub use status::ChallengeStatus;
mod plan_generator;
mod repository;
mod service;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        // literal paths BEFORE parameterised paths
        .service(handlers::list_public_challenges)     // GET  /challenges/public
        .service(handlers::list_challenges)            // GET  /challenges
        .service(handlers::create_challenge)           // POST /challenges
        .service(handlers::generate_challenge)         // POST /challenges/generate
        .service(handlers::get_challenge)              // GET  /challenges/{id}
        .service(handlers::update_challenge)           // PUT  /challenges/{id}
        .service(handlers::delete_challenge)           // DEL  /challenges/{id}
        .service(handlers::activate_challenge)         // POST /challenges/{id}/activate
        .service(handlers::opt_in_challenge)           // POST /challenges/{id}/opt-in
        .service(handlers::get_participants)           // GET  /challenges/{id}/participants
        .service(handlers::get_challenge_leaderboard)  // GET  /challenges/{id}/leaderboard
        .service(handlers::add_workout)
        .service(handlers::update_workout)
        .service(handlers::reorder_workout)
        .service(handlers::delete_workout)
        .service(handlers::add_requirement)
        .service(handlers::delete_requirement);
}
