//! Rate limiting guard.
use rocket::serde::json::Json;
use rocket_governor::{Method, Quota, RocketGovernable};

use crate::api::{ApiError, ApiErrorResponse};

pub struct RateLimit;

#[rocket::catch(429)]
pub async fn rate_limit() -> ApiErrorResponse {
    ApiErrorResponse::TooManyRequests {
        inner: (
            rocket::http::Status::TooManyRequests,
            Json(ApiError {
                message: "Rate limit exceeded".to_string(),
                details: None,
                status: 429,
            }),
        ),
    }
}

impl<'r> RocketGovernable<'r> for RateLimit {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_second(Self::nonzero(1u32))
    }
}
