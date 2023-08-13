use crate::AppState;
use axum::{
	extract::{Path, Query},
	http::StatusCode,
	response::IntoResponse,
	routing::get,
	Router,
};
use dbost_services::auth::{AuthService, CompleteAuthenticationError, StartAuthenticationError};
use serde::Deserialize;
use tracing::warn;

#[derive(Deserialize)]
struct LoginQuery {
	#[serde(default = "Default::default")]
	return_to: Option<String>,
}

async fn login(
	Path(provider): Path<String>,
	Query(query): Query<LoginQuery>,
	auth: AuthService,
) -> impl IntoResponse {
	match auth.login(&provider, query.return_to.as_deref().unwrap_or("/")) {
		Ok(response) => response.into_response(),
		Err(StartAuthenticationError::InvalidProvider(_)) => {
			(StatusCode::NOT_FOUND, "Not found").into_response()
		}
	}
}

async fn register(
	Path(provider): Path<String>,
	Query(query): Query<LoginQuery>,
	auth: AuthService,
) -> impl IntoResponse {
	match auth.register(&provider, query.return_to.as_deref().unwrap_or("/")) {
		Ok(response) => response.into_response(),
		Err(StartAuthenticationError::InvalidProvider(_)) => {
			(StatusCode::NOT_FOUND, "Not found").into_response()
		}
	}
}

#[derive(Deserialize)]
struct CallbackQuery {
	code: String,
	state: String,
}

async fn callback(
	Path(provider): Path<String>,
	Query(query): Query<CallbackQuery>,
	auth: AuthService,
) -> impl IntoResponse {
	match auth.callback(&provider, &query.code, &query.state).await {
		Ok(response) => response.into_response(),
		Err(CompleteAuthenticationError::InvalidProvider(_)) => {
			(StatusCode::NOT_FOUND, "Not found").into_response()
		}
		Err(CompleteAuthenticationError::LoginWindowClosed) => {
			(StatusCode::BAD_REQUEST, "Login window closed").into_response()
		}
		Err(CompleteAuthenticationError::InvalidUser) => {
			(StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
		}
		Err(CompleteAuthenticationError::UserNotFound) => {
			(StatusCode::NOT_FOUND, "User not found").into_response()
		}
		Err(e) => {
			warn!("Failed to complete authentication: {}", e);
			(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
		}
	}
}

async fn logout(auth: AuthService) -> impl IntoResponse {
	match auth.logout().await {
		Ok(response) => response.into_response(),
		Err(e) => {
			warn!("Failed to logout: {}", e);
			(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
		}
	}
}

pub fn router() -> Router<AppState> {
	Router::new()
		.route("/logout", get(logout))
		.route("/login/:provider", get(login))
		.route("/register/:provider", get(register))
		.route("/callback/:provider", get(callback))
}
