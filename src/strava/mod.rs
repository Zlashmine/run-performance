pub mod auth;
pub mod client;
pub mod sync;
pub mod webhook;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(auth::connect_handler)
        .service(auth::disconnect_handler)
        .service(auth::status_handler)
        .service(sync::sync_handler)
        .service(webhook::validate_webhook)
        .service(webhook::receive_event);
}
