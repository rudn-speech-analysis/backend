use axum::response::IntoResponse;

pub type AppResult<T> = Result<T, AppError>;
pub struct AppError(eyre::Report);

impl<T> From<T> for AppError
where
    T: Into<eyre::Report>,
{
    fn from(value: T) -> Self {
        AppError(value.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("{:?}", self.0),
        )
            .into_response()
    }
}
