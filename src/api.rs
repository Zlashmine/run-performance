use std::env;

use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::http::header;
use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer};
use sqlx::PgPool;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::achievements::models::{AchievementWithStatus, UnlockedAchievementSummary};
use crate::activities::models::{
    ActivitiesResponse, Activity, ActivityDetailResponse, HeatmapPoint, HeatmapQuery, TrackPoint,
    UploadForm, UploadResponse,
};
use crate::challenges::models::{
    ActivateChallengeRequest, AddRequirementRequest, Challenge, ChallengeDetail, ChallengeSummary,
    ChallengeWorkout, CreateChallengeRequest, CreateWorkoutRequest, GenerateChallengeRequest,
    LeaderboardEntry, LeaderboardResponse, ListChallengesParams, ListPublicChallengesParams,
    OptInRequest, ParticipantsResponse, ReorderWorkoutRequest, UpdateChallengeRequest,
    UpdateWorkoutRequest, WorkoutRequirement, WorkoutWithDetails, WorkoutLink,
};
use crate::challenges::ChallengeStatus;
use crate::users::models::{CreateUser, User};
use crate::personal_records::models::{PersonalRecordsResponse, PersonalRecordSummary, PrCategorySummary};
use crate::missions::handler::{MissionHistoryEntry, MissionHistoryResponse};
use crate::missions::common::CompletedMissionSummary;
use crate::monthly_missions::models::{MonthlyMission, MonthlyMissionsResponse};
use crate::weekly_missions::models::{WeeklyMission, WeeklyMissionsResponse};
use crate::xp::models::{UserXpResponse, XpEvent};
use crate::{achievements, activities, challenges, missions, monthly_missions, personal_records, strava, users, weekly_missions, xp};
use crate::strava::client::StravaClient;

#[derive(OpenApi)]
#[openapi(
    paths(
        activities::handlers::get_activities,
        activities::handlers::get_activity_detail,
        activities::handlers::get_trackpoints,
        activities::handlers::get_heatmap,
        activities::handlers::upload_files,
        users::handlers::get_user,
        users::handlers::create_user,
        challenges::handlers::list_challenges,
        challenges::handlers::create_challenge,
        challenges::handlers::get_challenge,
        challenges::handlers::update_challenge,
        challenges::handlers::delete_challenge,
        challenges::handlers::add_workout,
        challenges::handlers::update_workout,
        challenges::handlers::reorder_workout,
        challenges::handlers::delete_workout,
        challenges::handlers::add_requirement,
        challenges::handlers::delete_requirement,
        challenges::handlers::list_public_challenges,
        challenges::handlers::activate_challenge,
        challenges::handlers::opt_in_challenge,
        challenges::handlers::get_participants,
        challenges::handlers::get_challenge_leaderboard,
        challenges::handlers::generate_challenge,
        xp::handler::get_user_xp,
        achievements::handler::get_user_achievements,
        personal_records::handler::get_user_prs,
        weekly_missions::handler::get_weekly_missions,
        weekly_missions::handler::reroll_mission,
        monthly_missions::handler::get_monthly_missions,
        monthly_missions::handler::reroll_monthly_mission,
        missions::handler::get_mission_history,
        health,
    ),
    components(schemas(
        Activity,
        ActivitiesResponse,
        ActivityDetailResponse,
        TrackPoint,
        UploadForm,
        UploadResponse,
        HeatmapPoint,
        HeatmapQuery,
        User,
        CreateUser,
        Challenge,
        ChallengeSummary,
        ChallengeDetail,
        ChallengeWorkout,
        WorkoutRequirement,
        WorkoutLink,
        WorkoutWithDetails,
        CreateChallengeRequest,
        UpdateChallengeRequest,
        CreateWorkoutRequest,
        UpdateWorkoutRequest,
        ReorderWorkoutRequest,
        AddRequirementRequest,
        ListChallengesParams,
        ChallengeStatus,
        ActivateChallengeRequest,
        OptInRequest,
        ListPublicChallengesParams,
        ParticipantsResponse,
        LeaderboardEntry,
        LeaderboardResponse,
        GenerateChallengeRequest,
        UserXpResponse,
        XpEvent,
        AchievementWithStatus,
        UnlockedAchievementSummary,
        PersonalRecordsResponse,
        PersonalRecordSummary,
        PrCategorySummary,
        WeeklyMission,
        WeeklyMissionsResponse,
        MonthlyMission,
        MonthlyMissionsResponse,
        MissionHistoryEntry,
        MissionHistoryResponse,
        CompletedMissionSummary,
    )),
    tags(
        (name = "Activities", description = "Activity management"),
        (name = "Users",      description = "User management"),
        (name = "challenges", description = "Running challenges"),
    )
)]
struct ApiDoc;

/// Liveness probe.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

fn build_cors() -> Cors {
    let origins_env = env::var("CORS_ORIGINS").unwrap_or_default();
    let origins: Vec<String> = origins_env
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut cors = Cors::default()
        .allow_any_method()
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .max_age(3600);

    for origin in &origins {
        cors = cors.allowed_origin(origin);
    }

    cors
}

pub async fn run_api(db_pool: PgPool) -> std::io::Result<()> {
    info!("Starting server...");

    sqlx::migrate!()
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");

    info!("Database migrations applied successfully.");

    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid u16");

    // Rate-limit: allow 1 request per 2 seconds, burst of 20.
    // Tune via GOVERNOR_PER_SECOND / GOVERNOR_BURST env vars if needed.
    let governor_conf = GovernorConfigBuilder::default()
        .seconds_per_request(2)
        .burst_size(20)
        .finish()
        .unwrap();

    info!("Binding server to 0.0.0.0:{}", port);

    HttpServer::new(move || {
        // 1 MiB JSON payload limit (prevents oversized body attacks).
        let json_cfg = web::JsonConfig::default().limit(1_048_576);

        App::new()
            .wrap(Logger::default())
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(
                actix_web::middleware::DefaultHeaders::new()
                    .add((
                        header::STRICT_TRANSPORT_SECURITY,
                        "max-age=63072000; includeSubDomains; preload",
                    ))
                    .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
                    .add((header::X_FRAME_OPTIONS, "DENY"))
                    .add((header::X_XSS_PROTECTION, "1; mode=block")),
            )
            .wrap(Governor::new(&governor_conf))
            .wrap(build_cors()) // OUTERMOST: handles OPTIONS before rate-limiter can reject
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(StravaClient::new()))
            .app_data(json_cfg)
            .service(health)
            .configure(activities::configure)
            .configure(users::configure)
            .configure(challenges::configure)
            .configure(xp::configure)
            .configure(achievements::configure)
            .configure(personal_records::configure)
            .configure(weekly_missions::configure)
            .configure(monthly_missions::configure)
            .configure(missions::handler::configure)
            .configure(strava::configure)
            .service(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
