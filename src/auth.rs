use axum::{
	http::{Request, StatusCode},
	response::{IntoResponse, Response},
};
use futures::future::Either;
use std::{
	future::{ready, Future, Ready},
	sync::Arc,
	task::{Context, Poll},
};
use tower::{Layer, Service};

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
	type Service = AuthorizationService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthorizationService {
			api_key: self.api_key.clone(),
			inner,
		}
	}
}

#[derive(Clone)]
pub struct AuthorizationService<S> {
	api_key: Arc<str>,
	inner: S,
}

impl<S, B> Service<Request<B>> for AuthorizationService<S>
where
	S: Service<Request<B>, Response = Response>,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = Either<S::Future, Ready<<S::Future as Future>::Output>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: Request<B>) -> Self::Future {
		let api_key = req.headers().get("x-api-key").and_then(|h| h.to_str().ok());

		let authorized = api_key == Some(&*self.api_key);
		if authorized {
			Either::Left(self.inner.call(req))
		} else {
			let response = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
			Either::Right(ready(Ok(response)))
		}
	}
}
