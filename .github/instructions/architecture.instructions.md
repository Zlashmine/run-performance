---
applyTo: "src/**/*.rs"
---

# Architecture rules

## Module boundaries (strictly enforced)
| Layer | Allowed | Forbidden |
|---|---|---|
| `handlers.rs` | HTTP request/response, AppError, calls service | SQL, direct DB calls |
| `service.rs` | Business logic, calls repository | HttpResponse, actix types |
| `repository.rs` | SQL queries, sqlx, returns AppError | HttpResponse, actix types, business logic |
| `parser.rs` | Pure sync parsing | DB, HTTP, async |
| `models.rs` | Struct definitions, serde, utoipa derives | impl blocks with logic |

## configure() is mandatory
Every routable domain module MUST expose:
```rust
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/prefix")
            .route("...", web::get().to(handler_fn))
            ...
    );
}
```
`api.rs` links domains via `.configure(activities::configure).configure(users::configure)`.

## Domain module public surface
`mod.rs` exposes only what other modules need:
```rust
pub mod handlers;
pub mod models;
pub mod parser;   // (activities only)
mod repository;
mod service;
pub use handlers::{get_activities, upload_files, ...};
pub fn configure(cfg: &mut web::ServiceConfig) { ... }
```

## Aggregate domain is special
`aggregate/service.rs` contains **only pure functions** — no DB, no HTTP, no side-effects.
Arguments are slices of `Activity` structs already loaded from the DB.
This makes it trivially unit-testable without any test infrastructure.

## Repository patterns
- Every repository function accepts `&PgPool` as first argument.
- Return type is always `Result<T, AppError>`.
- Map sqlx errors via `From<sqlx::Error>` impl on `AppError` (see `error.rs`).
- Bulk inserts use `query_builder::QueryBuilder` or `UNNEST` patterns; prefer
  `ON CONFLICT … DO NOTHING` over pre-fetch deduplication.

## Handler pattern
```rust
pub async fn get_thing(
    db: web::Data<PgPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid UUID".into()))?;
    let result = service::get_thing(&db, id).await?;
    Ok(HttpResponse::Ok().json(result))
}
```
