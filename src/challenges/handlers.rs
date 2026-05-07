/// HTTP handlers for the challenges domain.
use actix_web::{delete, get, post, put, web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::{
    models::{
        ActivateChallengeRequest, AddRequirementRequest, CreateChallengeRequest,
        CreateWorkoutRequest, GenerateChallengeRequest, ListChallengesParams,
        ListPublicChallengesParams, OptInRequest, ReorderWorkoutRequest, UpdateChallengeRequest,
        UpdateWorkoutRequest,
    },
    service,
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str, label: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(s).map_err(|_| AppError::BadRequest(format!("Invalid {label} UUID")))
}

// ─── Challenges ───────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/challenges",
    params(
        ("user_id"  = Uuid,          Query, description = "Filter by user ID"),
        ("limit"    = Option<i64>,   Query, description = "Max results (1–100, default 20)"),
        ("offset"   = Option<i64>,   Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "Paginated list of challenges with progress counts",
         body = Vec<super::models::ChallengeSummary>, content_type = "application/json"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[get("/challenges")]
pub async fn list_challenges(
    query: web::Query<ListChallengesParams>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let result = service::list_challenges(db.get_ref(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    post,
    path = "/challenges",
    request_body = super::models::CreateChallengeRequest,
    responses(
        (status = 201, description = "Challenge created",
         body = super::models::Challenge, content_type = "application/json"),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[post("/challenges")]
pub async fn create_challenge(
    body: web::Json<CreateChallengeRequest>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let result = service::create_challenge(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

#[utoipa::path(
    get,
    path = "/challenges/{challenge_id}",
    params(
        ("challenge_id" = String, Path, description = "Challenge ID (UUID v4)")
    ),
    responses(
        (status = 200, description = "Challenge with workouts, requirements and links",
         body = super::models::ChallengeDetail, content_type = "application/json"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[get("/challenges/{challenge_id}")]
pub async fn get_challenge(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let challenge_id = parse_uuid(&path.into_inner(), "challenge_id")?;
    let result = service::get_challenge_detail(db.get_ref(), challenge_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    put,
    path = "/challenges/{challenge_id}",
    params(
        ("challenge_id" = String, Path, description = "Challenge ID (UUID v4)")
    ),
    request_body = super::models::UpdateChallengeRequest,
    responses(
        (status = 200, description = "Updated challenge",
         body = super::models::Challenge, content_type = "application/json"),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[put("/challenges/{challenge_id}")]
pub async fn update_challenge(
    path: web::Path<String>,
    body: web::Json<UpdateChallengeRequest>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let challenge_id = parse_uuid(&path.into_inner(), "challenge_id")?;
    let result = service::update_challenge(db.get_ref(), challenge_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    delete,
    path = "/challenges/{challenge_id}",
    params(
        ("challenge_id" = String, Path, description = "Challenge ID (UUID v4)")
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[delete("/challenges/{challenge_id}")]
pub async fn delete_challenge(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let challenge_id = parse_uuid(&path.into_inner(), "challenge_id")?;
    service::delete_challenge(db.get_ref(), challenge_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

// ─── Workouts ─────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/challenges/{challenge_id}/workouts",
    params(
        ("challenge_id" = String, Path, description = "Challenge ID (UUID v4)")
    ),
    request_body = super::models::CreateWorkoutRequest,
    responses(
        (status = 201, description = "Workout created",
         body = super::models::ChallengeWorkout, content_type = "application/json"),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Challenge not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[post("/challenges/{challenge_id}/workouts")]
pub async fn add_workout(
    path: web::Path<String>,
    body: web::Json<CreateWorkoutRequest>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let challenge_id = parse_uuid(&path.into_inner(), "challenge_id")?;
    let result = service::add_workout(db.get_ref(), challenge_id, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

#[utoipa::path(
    put,
    path = "/workouts/{workout_id}",
    params(
        ("workout_id" = String, Path, description = "Workout ID (UUID v4)")
    ),
    request_body = super::models::UpdateWorkoutRequest,
    responses(
        (status = 200, description = "Updated workout",
         body = super::models::ChallengeWorkout, content_type = "application/json"),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[put("/workouts/{workout_id}")]
pub async fn update_workout(
    path: web::Path<String>,
    body: web::Json<UpdateWorkoutRequest>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let workout_id = parse_uuid(&path.into_inner(), "workout_id")?;
    let result = service::update_workout(db.get_ref(), workout_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    put,
    path = "/workouts/{workout_id}/reorder",
    params(
        ("workout_id" = String, Path, description = "Workout ID (UUID v4)")
    ),
    request_body = super::models::ReorderWorkoutRequest,
    responses(
        (status = 200, description = "Workout moved to new position",
         body = super::models::ChallengeWorkout, content_type = "application/json"),
        (status = 400, description = "Invalid position"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[put("/workouts/{workout_id}/reorder")]
pub async fn reorder_workout(
    path: web::Path<String>,
    body: web::Json<ReorderWorkoutRequest>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let workout_id = parse_uuid(&path.into_inner(), "workout_id")?;
    let result = service::reorder_workout(db.get_ref(), workout_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    delete,
    path = "/workouts/{workout_id}",
    params(
        ("workout_id" = String, Path, description = "Workout ID (UUID v4)")
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[delete("/workouts/{workout_id}")]
pub async fn delete_workout(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let workout_id = parse_uuid(&path.into_inner(), "workout_id")?;
    service::delete_workout(db.get_ref(), workout_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

// ─── Requirements ─────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/workouts/{workout_id}/requirements",
    params(
        ("workout_id" = String, Path, description = "Workout ID (UUID v4)")
    ),
    request_body = super::models::AddRequirementRequest,
    responses(
        (status = 201, description = "Requirement added",
         body = super::models::WorkoutRequirement, content_type = "application/json"),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Workout not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[post("/workouts/{workout_id}/requirements")]
pub async fn add_requirement(
    path: web::Path<String>,
    body: web::Json<AddRequirementRequest>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let workout_id = parse_uuid(&path.into_inner(), "workout_id")?;
    let result = service::add_requirement(db.get_ref(), workout_id, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

#[utoipa::path(
    delete,
    path = "/requirements/{requirement_id}",
    params(
        ("requirement_id" = String, Path, description = "Requirement ID (UUID v4)")
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[delete("/requirements/{requirement_id}")]
pub async fn delete_requirement(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let requirement_id = parse_uuid(&path.into_inner(), "requirement_id")?;
    service::delete_requirement(db.get_ref(), requirement_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

// ─── Public challenges & lifecycle ────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/challenges/public",
    params(
        ("limit"  = Option<i64>, Query, description = "Max results (1-100, default 20)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "Publicly visible challenges with workout and participant counts",
         body = Vec<super::models::ChallengeSummary>, content_type = "application/json"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[get("/challenges/public")]
pub async fn list_public_challenges(
    query: web::Query<ListPublicChallengesParams>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let result = service::get_public_challenges(db.get_ref(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    post,
    path = "/challenges/{challenge_id}/activate",
    params(
        ("challenge_id" = String, Path, description = "Challenge ID (UUID v4)")
    ),
    request_body = super::models::ActivateChallengeRequest,
    responses(
        (status = 200, description = "Challenge activated",
         body = super::models::Challenge, content_type = "application/json"),
        (status = 400, description = "Challenge not in Draft state"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[post("/challenges/{challenge_id}/activate")]
pub async fn activate_challenge(
    path: web::Path<String>,
    body: web::Json<ActivateChallengeRequest>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let challenge_id = parse_uuid(&path.into_inner(), "challenge_id")?;
    let result =
        service::activate_challenge(db.get_ref(), challenge_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    post,
    path = "/challenges/{challenge_id}/opt-in",
    params(
        ("challenge_id" = String, Path, description = "Challenge ID (UUID v4)")
    ),
    request_body = super::models::OptInRequest,
    responses(
        (status = 201, description = "Opted in — clone challenge returned",
         body = super::models::Challenge, content_type = "application/json"),
        (status = 400, description = "Not public, expired, or already opted in"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[post("/challenges/{challenge_id}/opt-in")]
pub async fn opt_in_challenge(
    path: web::Path<String>,
    body: web::Json<OptInRequest>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let challenge_id = parse_uuid(&path.into_inner(), "challenge_id")?;
    let result = service::opt_in_challenge(db.get_ref(), challenge_id, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

#[utoipa::path(
    get,
    path = "/challenges/{challenge_id}/participants",
    params(
        ("challenge_id" = String, Path, description = "Challenge ID (UUID v4)"),
        ("limit"        = Option<i64>, Query, description = "Max results (1-100, default 20)"),
        ("offset"       = Option<i64>, Query, description = "Pagination offset (default 0)"),
    ),
    responses(
        (status = 200, description = "Participant count and list",
         body = super::models::ParticipantsResponse, content_type = "application/json"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "challenges"
)]
#[get("/challenges/{challenge_id}/participants")]
pub async fn get_participants(
    path: web::Path<String>,
    query: web::Query<ListPublicChallengesParams>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let challenge_id = parse_uuid(&path.into_inner(), "challenge_id")?;
    let result =
        service::get_participants(db.get_ref(), challenge_id, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

// ─── Leaderboard ─────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/challenges/{challenge_id}/leaderboard",
    params(("challenge_id" = String, Path, description = "Challenge ID")),
    responses(
        (status = 200, body = super::models::LeaderboardResponse, content_type = "application/json"),
        (status = 400, description = "Not a public challenge"),
        (status = 404, description = "Challenge not found"),
    ),
    tag = "challenges"
)]
#[get("/challenges/{challenge_id}/leaderboard")]
pub async fn get_challenge_leaderboard(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let challenge_id = parse_uuid(&path.into_inner(), "challenge_id")?;
    let response = super::repository::get_leaderboard(db.get_ref(), challenge_id).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ─── Training Plan Generation ─────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/challenges/generate",
    request_body = GenerateChallengeRequest,
    responses(
        (status = 201, body = crate::challenges::models::Challenge,
         content_type = "application/json"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "challenges"
)]
#[post("/challenges/generate")]
pub async fn generate_challenge(
    db: web::Data<PgPool>,
    body: web::Json<GenerateChallengeRequest>,
) -> Result<HttpResponse, AppError> {
    let challenge = service::generate_challenge(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(challenge))
}
