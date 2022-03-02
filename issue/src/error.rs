use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum AppError {
    BadRequest,
    Unauthorized,
    NotFound,
}

pub enum IssueError {
    BadRequest,
    Unauthorized,
    NotFound,
}

impl From<IssueError> for AppError {
    fn from(inner: IssueError) -> Self {
        match inner {
            IssueError::BadRequest => AppError::BadRequest,
            IssueError::Unauthorized => AppError::Unauthorized,
            IssueError::NotFound => AppError::NotFound,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_mesage) = match self {
            AppError::BadRequest => (StatusCode::BAD_REQUEST, "Bad Request"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not Found"),
        };

        let body = Json(json!({ "error": error_mesage }));

        (status, body).into_response()
    }
}

