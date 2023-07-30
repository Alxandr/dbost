use async_trait::async_trait;
use futures::{channel::oneshot, future::Shared, FutureExt};
use reqwest::{
	header::{self, HeaderValue},
	Body, Method, Request, Response,
};
use reqwest_middleware::{Middleware, Next};
use serde::{Deserialize, Serialize};
use std::{
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, Mutex, MutexGuard,
	},
	time::{Duration, Instant},
};
use task_local_extensions::Extensions;
use tracing::{debug, info, instrument};

use crate::TvDbUrl;

enum ApiToken {
	Missing,
	Present {
		token: HeaderValue,
		expires_at: Instant,
	},
	Pending {
		generation: usize,
		future: Shared<oneshot::Receiver<HeaderValue>>,
	},
	Poisoned,
}

impl Default for ApiToken {
	fn default() -> Self {
		Self::Missing
	}
}

impl ApiToken {
	// tokens last for 1 month - knock down to 20 days to have good margin
	const DURATION: Duration = Duration::from_secs(60 * 60 * 24 * 20);

	fn not_expired(expires_at: Instant) -> bool {
		expires_at > Instant::now()
	}
}

pub(crate) struct AuthMiddleware {
	api_key: String,
	user_pin: String,
	generation: AtomicUsize,
	token: Mutex<Arc<ApiToken>>,
}

impl AuthMiddleware {
	pub(crate) fn new(api_key: String, user_pin: String) -> Self {
		Self {
			api_key,
			user_pin,
			generation: AtomicUsize::new(0),
			token: Default::default(),
		}
	}
}

enum SyncTokenResult {
	Present(HeaderValue),
	Pending(usize, Shared<oneshot::Receiver<HeaderValue>>),
	Missing(usize, oneshot::Sender<HeaderValue>),
}

impl AuthMiddleware {
	fn get_token(mut guard: MutexGuard<Arc<ApiToken>>, generation: &AtomicUsize) -> SyncTokenResult {
		match &**guard {
			ApiToken::Poisoned => panic!("Poisoned token"),
			ApiToken::Present { token, expires_at } if ApiToken::not_expired(*expires_at) => {
				SyncTokenResult::Present(token.clone())
			}
			ApiToken::Pending { generation, future } => {
				SyncTokenResult::Pending(*generation, future.clone())
			}
			ApiToken::Missing | ApiToken::Present { .. } => {
				let (tx, rx) = oneshot::channel();
				let future = rx.shared();
				let generation = generation.fetch_add(1, Ordering::Relaxed);
				*guard = Arc::new(ApiToken::Pending { generation, future });
				SyncTokenResult::Missing(generation, tx)
			}
		}
	}

	#[instrument(skip_all)]
	async fn get_or_fetch_token<'a>(
		&self,
		next: Next<'a>,
		extensions: &mut Extensions,
	) -> HeaderValue {
		loop {
			let sync_result = Self::get_token(self.token.lock().unwrap(), &self.generation);
			let result = match sync_result {
				SyncTokenResult::Present(token) => {
					debug!("token present");
					Some(token)
				}

				SyncTokenResult::Pending(gen, future) => match future.await {
					Ok(token) => Some(token),
					Err(_) => {
						{
							let mut guard = self.token.lock().unwrap();
							if matches!(**guard, ApiToken::Pending { generation, .. } if gen == generation) {
								*guard = Arc::new(ApiToken::Missing);
							}
						}

						None
					}
				},

				SyncTokenResult::Missing(generation, tx) => Some(
					self
						.fetch_token(tx, next.clone(), extensions, generation)
						.await,
				),
			};

			match result {
				Some(token) => break token,
				None => continue,
			}
		}
	}

	#[instrument(skip_all)]
	async fn fetch_token<'a>(
		&self,
		tx: oneshot::Sender<HeaderValue>,
		next: Next<'a>,
		extensions: &mut Extensions,
		_generation: usize,
	) -> HeaderValue {
		#[derive(Serialize)]
		struct LoginRequest<'a> {
			apikey: &'a str,
			pin: &'a str,
		}

		#[derive(Deserialize)]
		struct LoginResponse {
			data: LoginResponseData,
		}

		#[derive(Deserialize)]
		struct LoginResponseData {
			token: String,
		}

		let mut login_request = Request::new(Method::POST, TvDbUrl::Login.into_url());
		login_request.headers_mut().insert(
			header::CONTENT_TYPE,
			header::HeaderValue::from_static("application/json"),
		);

		let body = serde_json::to_vec(&LoginRequest {
			apikey: &self.api_key,
			pin: &self.user_pin,
		})
		.unwrap();

		*login_request.body_mut() = Some(Body::from(body));

		debug!("sending login request");
		let response = match next.run(login_request, extensions).await {
			Ok(response) => response,
			Err(e) => {
				*self.token.lock().unwrap() = Arc::new(ApiToken::Poisoned);
				panic!("Error fetching token - request error: {:#?}", e)
			}
		};

		debug!(status = %response.status(), "got login response");
		if !response.status().is_success() {
			*self.token.lock().unwrap() = Arc::new(ApiToken::Poisoned);
			let status = response.status();
			// let headers = response.headers().clone();
			let body = response.text().await;
			panic!(
				"Error fetching token - bad status code: {}\n{:?}",
				status, body
			);
		}

		let response: LoginResponse = match response.json().await {
			Ok(response) => response,
			Err(e) => {
				*self.token.lock().unwrap() = Arc::new(ApiToken::Poisoned);
				panic!("Error fetching token - failed to deserialize: {:#?}", e)
			}
		};

		let token = HeaderValue::from_str(&format!("Bearer {}", &response.data.token)).unwrap();
		let expires_at = Instant::now() + ApiToken::DURATION;
		info!("fetched new tvdb token");

		*self.token.lock().unwrap() = Arc::new(ApiToken::Present {
			token: token.clone(),
			expires_at,
		});
		let _ = tx.send(token.clone());

		token
	}
}

#[async_trait]
impl Middleware for AuthMiddleware {
	async fn handle(
		&self,
		mut req: Request,
		extensions: &mut Extensions,
		next: Next<'_>,
	) -> reqwest_middleware::Result<Response> {
		let token = self.get_or_fetch_token(next.clone(), extensions).await;

		req.headers_mut().append(header::AUTHORIZATION, token);
		next.run(req, extensions).await
	}
}
