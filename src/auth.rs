use axum::{
	body::BoxBody,
	http::{Request, StatusCode},
	response::{IntoResponse, Response},
};
use std::{
	future::{ready, Ready},
	sync::Arc,
};
use tower::Layer;
use tower_http::auth::{AsyncAuthorizeRequest, AsyncRequireAuthorization};

#[derive(Clone)]
pub struct AuthorizationLayer {
	api_key: Arc<str>,
}

impl AuthorizationLayer {
	pub fn new(api_key: impl Into<Arc<str>>) -> Self {
		Self {
			api_key: api_key.into(),
		}
	}
}

impl<S> Layer<S> for AuthorizationLayer {
	type Service = AsyncRequireAuthorization<S, AuthorizationHandler>;

	fn layer(&self, inner: S) -> Self::Service {
		AsyncRequireAuthorization::new(
			inner,
			AuthorizationHandler {
				api_key: self.api_key.clone(),
			},
		)
	}
}

#[derive(Clone)]
pub struct AuthorizationHandler {
	api_key: Arc<str>,
}

impl<B> AsyncAuthorizeRequest<B> for AuthorizationHandler {
	type RequestBody = B;
	type ResponseBody = BoxBody;
	type Future = Ready<Result<Request<B>, Response<Self::ResponseBody>>>;

	fn authorize(&mut self, request: Request<B>) -> Self::Future {
		let api_key = request
			.headers()
			.get("x-api-key")
			.and_then(|h| h.to_str().ok());

		let authorized = api_key == Some(&*self.api_key);
		if authorized {
			ready(Ok(request))
		} else {
			let response = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
			ready(Err(response))
		}
	}
}
