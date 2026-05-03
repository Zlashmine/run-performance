# Copilot Instructions — run-performance (Rust backend)

## Project overview
`run-performance` is a Rust/Actix-web REST API for tracking running activities.
It reads activity data from Garmin CSV exports and GPX files, stores them in
PostgreSQL via sqlx, and returns pre-computed aggregations.

## Stack
- **Rust 2021**, Actix-web 4.x, sqlx 0.8 (async PostgreSQL)
- **Auth / rate-limit**: actix-governor
- **OpenAPI**: utoipa 5.x + utoipa-swagger-ui
- **Utilities**: uuid, chrono, gpx, sanitize-filename, validator, tracing

## Architecture
```
handler → service → repository
```
Each domain (`activities`, `users`, `aggregate`) exposes a `configure()` fn
that `api.rs` calls.  **No SQL in handlers, no HTTP in repositories.**

### Domain layout
```
src/
  activities/
    handlers.rs   ← HTTP only, delegates to service
    service.rs    ← business logic
    repository.rs ← SQL only
    parser.rs     ← synchronous CSV / GPX parsing (no async)
    models.rs     ← data structs (no impl blocks for parsing)
    mod.rs        ← re-exports + configure()
  users/           ← same pattern
  aggregate/
    service.rs    ← pure aggregation functions (no DB, no HTTP)
    scoring.rs    ← scoring config and helpers
    models.rs
    mod.rs
  api.rs          ← App builder, CORS, rate-limit, OpenAPI
  db.rs           ← PgPool init (reads DB_* env vars)
  error.rs        ← AppError enum implementing ResponseError
  lib.rs          ← module declarations
  main.rs         ← #[actix_web::main] entry point only
```

## Key conventions
1. **Error handling**: return `Result<HttpResponse, AppError>` from all handlers.
   Use `AppError::NotFound`, `AppError::BadRequest(msg)`, `AppError::Unauthorized`,
   `AppError::Internal`.  Never expose raw sqlx errors to the client.
2. **UUID parsing**: parse exactly once in the handler with
   `Uuid::parse_str(&s).map_err(|_| AppError::BadRequest("Invalid UUID".into()))?`
3. **DB deduplication**: activities use `ON CONFLICT (user_id, date) DO NOTHING`.
   Trackpoints use `ON CONFLICT (activity_id, time) DO NOTHING`.
4. **TrackPoint fields**: `latitude: f64`, `longitude: f64`, `time: DateTime<Utc>`.
   DB columns are `lat DOUBLE PRECISION`, `lon DOUBLE PRECISION`, `time TIMESTAMPTZ`.
   Use column aliases in SELECT: `lat AS latitude, lon AS longitude`.
5. **Logging**: use `tracing::{info!, warn!, error!}` — never `println!` or `eprintln!`.
6. **Parsing functions** in `parser.rs` are synchronous (pure CPU, no I/O).
   Do not add `.await` to them.
7. **CORS**: controlled by `CORS_ORIGINS` env var (comma-separated origins).
   Never use `allow_any_origin()` in production.
8. **Secrets**: never `COPY .env` into Docker images.  Use `environment:` in
   docker-compose or inject at runtime.

## Environment variables
| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | — | PostgreSQL connection string |
| `PORT` | `8080` | Server listen port |
| `RUST_LOG` | — | Tracing filter (e.g. `info`) |
| `CORS_ORIGINS` | — | Comma-separated allowed origins |
| `DB_MAX_CONNECTIONS` | `5` | Pool max connections |
| `DB_MIN_CONNECTIONS` | `1` | Pool min idle connections |
| `DB_CONNECT_TIMEOUT_SECS` | `5` | Pool connect timeout |

## Testing
- Unit tests (pure logic, no DB): run with `cargo test`
- Integration tests need a running PostgreSQL; use `.env.test` and
  `cargo test --test <filename>`
- Activity parsing tests live in `tests/activity_utils_tests.rs` and are **sync**
  (no `#[tokio::test]`).
- Aggregation tests are in `tests/aggregations_tests.rs` and are also **sync**.
