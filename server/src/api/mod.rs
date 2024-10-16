//! # API

use konarr::KonarrError;
use rocket::{
    http::Status,
    response::{self, Responder},
    serde::json::Json,
    Request,
};

use crate::error::KonarrServerError;

pub mod admin;
pub mod auth;
pub mod base;
pub mod dependencies;
pub mod projects;
pub mod security;
pub mod snapshots;
pub mod websock;

/// API Response Wrapper
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiResponse<T>
where
    T: serde::Serialize,
{
    pub data: T,
    pub total: u32,
    pub pages: u32,
}

impl<T> ApiResponse<T>
where
    T: serde::Serialize,
{
    pub fn new(data: T, total: u32, pages: u32) -> Self {
        Self { data, total, pages }
    }
}

#[derive(Responder)]
pub enum ApiErrorResponse {
    #[response(status = 401, content_type = "json")]
    Unauthorized { inner: (Status, Json<ApiError>) },
    #[response(status = 404, content_type = "json")]
    NotFound { inner: (Status, Json<ApiError>) },
    #[response(status = 500, content_type = "json")]
    InternalServerError { inner: (Status, Json<ApiError>) },
    #[response(status = 429, content_type = "json")]
    TooManyRequests { inner: (Status, Json<ApiError>) },
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiError {
    pub message: String,
    #[cfg(debug_assertions)]
    pub details: String,
    pub status: i16,
}

pub type ApiResult<T> = Result<Json<T>, KonarrServerError>;

impl<'r> Responder<'r, 'r> for KonarrServerError {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'r> {
        let status = match self {
            KonarrServerError::KonarrError(KonarrError::GeekOrm(geekorm::Error::NoRowsFound))
            | KonarrServerError::GeekOrmError(geekorm::Error::NoRowsFound) => Status::NotFound,
            _ => Status::InternalServerError,
        };

        ApiErrorResponse::InternalServerError {
            inner: (
                status,
                Json(ApiError {
                    message: "Internal Server Error".to_string(),
                    #[cfg(debug_assertions)]
                    details: self.to_string(),
                    status: status.code as i16,
                }),
            ),
        }
        .respond_to(request)
    }
}

impl From<ApiError> for ApiErrorResponse {
    fn from(value: ApiError) -> ApiErrorResponse {
        match value.status {
            401 => ApiErrorResponse::Unauthorized {
                inner: (Status::Unauthorized, Json(value)),
            },
            404 => ApiErrorResponse::NotFound {
                inner: (Status::NotFound, Json(value)),
            },
            _ => ApiErrorResponse::InternalServerError {
                inner: (Status::InternalServerError, Json(value)),
            },
        }
    }
}

impl From<konarr::KonarrError> for ApiError {
    fn from(value: konarr::KonarrError) -> ApiError {
        ApiError {
            message: "Internal Server Error".to_string(),
            #[cfg(debug_assertions)]
            details: value.to_string(),
            status: 500,
        }
    }
}

impl From<geekorm::Error> for ApiError {
    fn from(error: geekorm::Error) -> Self {
        ApiError {
            message: "Internal Server Error".to_string(),
            #[cfg(debug_assertions)]
            details: error.to_string(),
            status: 500,
        }
    }
}

impl From<libsql::Error> for ApiError {
    fn from(error: libsql::Error) -> Self {
        ApiError {
            message: "Internal Server Error".to_string(),
            #[cfg(debug_assertions)]
            details: error.to_string(),
            status: 500,
        }
    }
}
