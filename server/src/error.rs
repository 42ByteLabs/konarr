use konarr::KonarrError;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(unused)]
pub enum KonarrServerError {
    /// Database Connection Error
    #[error("Failed to connect to the database")]
    DatabaseConnectionError,
    /// Dependency Fetch Error
    #[error("Failed to fetch dependency")]
    DependencyFetchError,
    /// Dependency Not Found Error
    #[error("Dependency {0} not found")]
    DependencyNotFoundError(i32),
    /// Dependency Fetch Error
    #[error("Failed to fetch project")]
    ProjectFetchError,
    /// Project Not Found Error
    #[error("Project {0} not found")]
    ProjectNotFoundError(i32),
    /// Unauthorized Error
    #[error("Unauthorized")]
    Unauthorized,
    /// Internal Server Error
    #[error("Internal Server Error")]
    InternalServerError,
    /// Konarr Internal Error
    #[error("Konarr Error: {0}")]
    KonarrError(#[from] KonarrError),

    /// Database Error (generic)
    #[error("Database Error: {0}")]
    DatabaseError(#[from] libsql::Error),
    /// ORM Error
    #[error("GeekOrm Error: {0}")]
    GeekOrmError(#[from] geekorm::Error),
}
