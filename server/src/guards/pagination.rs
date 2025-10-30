use geekorm::prelude::*;
use rocket::{
    Request,
    request::{self, FromRequest, Outcome},
};

/// Pagination request guard
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    /// Current page number (0-based)
    pub page: u32,
    /// Number of items per page
    pub limit: u32,
}

impl Pagination {
    /// Default page number
    pub const DEFAULT_PAGE: u32 = 0;
    /// Default limit
    pub const DEFAULT_LIMIT: u32 = 20;
    /// Maximum limit
    pub const MAX_LIMIT: u32 = 100;

    /// Create a new Pagination instance with validated values
    pub fn new(page: Option<u32>, limit: Option<u32>) -> Self {
        let page = page.unwrap_or(Self::DEFAULT_PAGE);
        let limit = limit
            .unwrap_or(Self::DEFAULT_LIMIT)
            .clamp(1, Self::MAX_LIMIT);

        Self { page, limit }
    }

    /// Get the Page representation
    pub fn page(&self) -> Page {
        Page::from((self.page, self.limit))
    }

    /// Get the Page representation with total count
    pub fn page_with_total(&self, total: u32) -> Page {
        Page::from((Some(self.page), Some(self.limit), total))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Pagination {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let page = req.query_value::<u32>("page").and_then(|r| r.ok());

        let limit = req.query_value::<u32>("limit").and_then(|r| r.ok());

        Outcome::Success(Pagination::new(page, limit))
    }
}
