use actix_web::{HttpResponse, ResponseError};
use std::fmt;

/// Shared application error type.
///
/// All handlers return `Result<HttpResponse, AppError>`.
/// Actix-web calls `error_response()` to convert this into an HTTP response.
/// Internal error details are **never** sent to the client — only generic phrases.
#[derive(Debug)]
pub enum AppError {
    NotFound,
    BadRequest(String),
    #[allow(dead_code)]
    Unauthorized,
    Internal,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound => write!(f, "Not Found"),
            AppError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            AppError::Unauthorized => write!(f, "Unauthorized"),
            AppError::Internal => write!(f, "Internal Server Error"),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound => HttpResponse::NotFound().finish(),
            AppError::BadRequest(msg) => HttpResponse::BadRequest().body(msg.clone()),
            AppError::Unauthorized => HttpResponse::Unauthorized().finish(),
            AppError::Internal => HttpResponse::InternalServerError().finish(),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound,
            _ => {
                tracing::error!("Database error: {}", e);
                AppError::Internal
            }
        }
    }
}
