use super::InternalApiState;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};

use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UserGet {
    id: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum UserGetError {
    #[error("user not found")]
    UserNotFound,
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for UserGetError {
    fn into_response(self) -> axum::response::Response {
        match self {
            UserGetError::UserNotFound => (StatusCode::NOT_FOUND, "user not found").into_response(),
            UserGetError::Sqlx(err) => {
                tracing::error!(?err, "sqlx error");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn user_get(
    State(state): State<InternalApiState>,
    Json(body): Json<UserGet>,
) -> Result<Json<serde_json::Value>, UserGetError> {
    let rec = sqlx::query!(
        r#"
        SELECT id, name, email FROM users WHERE id = $1
        "#,
        body.id
    )
    .fetch_optional(&state.app_cx.db())
    .await?;

    match rec {
        Some(user) => {
            let user_info = json!({
                "id": user.id.to_string(),
                "name": user.name,
                "email": user.email
            });
            Ok(Json(user_info))
        }
        None => Err(UserGetError::UserNotFound),
    }
}
