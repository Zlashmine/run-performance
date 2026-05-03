---
applyTo: "src/**/*.rs,Dockerfile,docker-compose.yml"
---

# Security rules

## CORS
- NEVER use `Cors::default()` or `.allow_any_origin()`.
- Always read from the `CORS_ORIGINS` environment variable (comma-separated).
- If `CORS_ORIGINS` is empty or unset, deny all cross-origin requests.

```rust
// CORRECT
let origins: Vec<String> = std::env::var("CORS_ORIGINS")
    .unwrap_or_default()
    .split(',')
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .collect();
```

## Secrets and .env files
- Never `COPY .env` or `COPY .env.*` in a Dockerfile.
- Never mount `.env` as a Docker volume in production.
- Pass secrets via `environment:` in docker-compose or runtime secret managers.
- The `.dockerignore` file must exclude `.env`, `.env.*`, and `target/`.

## Error leaking
- Never return raw `sqlx::Error` text to the client.
- `From<sqlx::Error>` in `error.rs` maps `RowNotFound → AppError::NotFound`
  and everything else to `AppError::Internal` (which sends only `"Internal server error"`).
- `AppError::Internal` must never include DB details in the HTTP response body.

## Rate limiting
- `actix-governor` is enabled in `api.rs` at 1 request per 2 seconds, burst 20.
- Do not disable or comment out the `Governor` middleware.

## UUID input validation
- All UUID path params must be parsed in the handler with a hard 400 on failure.
- Never pass raw untrusted strings directly to SQL; always use sqlx bind parameters.

## File upload security
- Filenames from multipart must be sanitised with `sanitize_filename::sanitize()`
  before use in any file system or DB operation.
- Enforce content-type and file extension checks before processing uploads.

## SQL injection prevention
- Always use sqlx `query!` / `query_as!` macros or `QueryBuilder::push_bind()`.
- Never concatenate user input directly into SQL strings.

## JSON payload size
- `web::JsonConfig::default().limit(1_048_576)` (1 MiB) is set in `api.rs`.
  Do not raise this limit without a documented justification.
