use axum::extract::FromRequestParts;
use axum_extra::extract::Host;

use crate::result::AppError;

pub struct UrlGenerator(Host);

impl UrlGenerator {
    pub fn url(&self, path: impl ToString) -> String {
        format!("http://{}{}", self.0.0, path.to_string())
    }
}

impl<S> FromRequestParts<S> for UrlGenerator
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let host = Host::from_request_parts(parts, state).await?;
        Ok(UrlGenerator(host))
    }
}
