---
applyTo: "tests/**/*.rs,src/**/*_test*.rs"
---

# Testing rules

## Test categories
| File | Type | Needs DB? | Async? |
|---|---|---|---|
| `tests/activity_utils_tests.rs` | Unit — CSV/GPX parsing | No | **No** |
| `tests/aggregations_tests.rs` | Unit — pure aggregation | No | **No** |
| `tests/activities_tests.rs` | Integration — HTTP handlers | Yes | Yes (`#[tokio::test]`) |
| `tests/users_tests.rs` | Integration — HTTP handlers | Yes | Yes (`#[tokio::test]`) |

## Parser tests are synchronous
`parser::parse_csv_row` and `parser::parse_gpx` are not async.
Do NOT annotate parser tests with `#[tokio::test]`, and do NOT call `.await`.

```rust
// CORRECT
#[test]
fn test_valid_csv_row() {
    let result = activities::parser::parse_csv_row(ROW, USER_ID);
    assert!(result.is_ok());
}
```

## Aggregate tests are pure
Functions in `aggregate::service` take slices, return structs — no DB, no HTTP.
Use plain `#[test]` and call directly.

## Integration test requirements
- Require a running PostgreSQL instance.
- Read `DATABASE_URL` from `.env.test` (not `.env`).
- Use `sqlx::PgPool::connect()` or the `db::create_pool()` helper.
- Clean up inserted rows in each test (use unique IDs or rollback transactions).

## Handler import convention
Import handlers by module path — not from the crate root:
```rust
// CORRECT
use run_performance::activities::handlers::{get_activities, get_trackpoints};
use run_performance::users::handlers::{create_user, get_user};

// WRONG (don't rely on wildcard re-exports from crate root)
use run_performance::get_activities;
```

## URL conventions
Activity routes are nested under users:
```
GET /users/{user_id}/activities
GET /activities/{activity_id}
GET /activities/{activity_id}/trackpoints
POST /activities/upload/{user_id}
```
User routes:
```
GET /users/{user_id}
POST /users
```

## UUID error assertions
An invalid UUID in a path (e.g. `"invalid-uuid"`) must return **400 Bad Request**,
not 404 or 500. Assert `status == 400` in handler integration tests.
