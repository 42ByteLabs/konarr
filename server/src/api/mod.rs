//! # API

use konarr::KonarrError;
use rocket::{
    Request,
    http::Status,
    response::{self, Responder},
    serde::json::Json,
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
    /// Total amount
    pub total: u32,
    /// Count of the search results
    pub count: u32,
    /// Page count
    pub pages: u32,
}

impl<T> ApiResponse<T>
where
    T: serde::Serialize,
{
    pub fn new(data: T, total: u32, pages: u32) -> Self {
        Self {
            data,
            total,
            count: total,
            pages,
        }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub status: i16,
}

pub type ApiResult<T> = Result<Json<T>, KonarrServerError>;

impl<'r> Responder<'r, 'r> for KonarrServerError {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'r> {
        match self {
            // Not Found
            KonarrServerError::GeekOrmError(geekorm::Error::NoRowsFound { query: _ })
            | KonarrServerError::KonarrError(KonarrError::GeekOrm(geekorm::Error::NoRowsFound {
                query: _,
            })) => ApiErrorResponse::NotFound {
                inner: (
                    Status::NotFound,
                    Json(ApiError {
                        message: "Not Found".to_string(),
                        details: Some(self.to_string()),
                        status: 404,
                    }),
                ),
            },
            // Unauthorized
            KonarrServerError::Unauthorized
            | KonarrServerError::KonarrError(KonarrError::AuthenticationError(_))
            | KonarrServerError::KonarrError(KonarrError::Unauthorized) => {
                ApiErrorResponse::Unauthorized {
                    inner: (
                        Status::Unauthorized,
                        Json(ApiError {
                            message: "Unauthorized".to_string(),
                            details: Some(self.to_string()),
                            status: 401,
                        }),
                    ),
                }
            }
            _ => ApiErrorResponse::InternalServerError {
                inner: (
                    Status::InternalServerError,
                    Json(ApiError {
                        message: "Internal Server Error".to_string(),
                        details: Some(self.to_string()),
                        status: 500,
                    }),
                ),
            },
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
        // We only want to include the details in the error message if the log level is set to debug
        let details: Option<String> = if log::max_level().to_string().as_str() == "debug" {
            Some(value.to_string())
        } else {
            None
        };

        match value {
            konarr::KonarrError::GeekOrm(geekorm::Error::NoRowsFound { query: _ }) => ApiError {
                message: "Not Found".to_string(),
                details,
                status: 404,
            },
            konarr::KonarrError::Unauthorized | konarr::KonarrError::AuthenticationError(_) => {
                ApiError {
                    message: "Unauthorized".to_string(),
                    details,
                    status: 401,
                }
            }
            _ => ApiError {
                message: "Internal Server Error".to_string(),
                details,
                status: 500,
            },
        }
    }
}
