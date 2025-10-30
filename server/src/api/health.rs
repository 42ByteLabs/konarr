//! # Health Check API

use crate::{AppState, api::ApiResult};
use konarr::{KonarrError, models::ProjectStatus};
use rocket::{State, serde::json::Json};

#[get("/")]
pub async fn health(state: &State<AppState>) -> ApiResult<serde_json::Value> {
    // Database check
    let connection = state.database.acquire().await;
    ProjectStatus::count_active(&connection)
        .await
        .map_err(|e| KonarrError::DatabaseError {
            backend: state.database.get_database_type().to_string(),
            error: e.to_string(),
        })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Konarr is running"
    })))
}
