mod store;

use arc_swap::ArcSwapOption;
use async_trait::async_trait;
use axum::{
	extract::FromRequestParts,
	response::{IntoResponse, Response},
};
use cookie::{
	time::{error::ComponentRange, OffsetDateTime},
	Cookie, CookieBuilder, Key, SameSite,
};
use dbost_entities::{session, user};
use dbost_utils::OffsetDateTimeExt;
use futures::{future::BoxFuture, FutureExt};
use http::{header, request, HeaderValue, Request, StatusCode};
use sea_orm::{ActiveValue, DatabaseConnection, DbErr, EntityTrait};
use std::{
	borrow::Cow,
	convert::Infallible,
	str::FromStr,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};
use thiserror::Error;
use time::Duration;
use tower_layer::Layer;
use tower_service::Service;
use tracing::{debug, error, info_span, instrument, Instrument};
use uuid::Uuid;

pub use store::CookieStore;

#[derive(Clone, Debug)]
pub struct CookieConfig {
	pub secure: bool,
	pub domain: Option<String>,
	pub path: String,
}

#[derive(Error, Debug)]
pub enum SessionError {
	#[error(transparent)]
	ExpiryConversion(#[from] ComponentRange),

	#[error(transparent)]
	DbError(#[from] DbErr),
}

struct SessionInner {
	id: Uuid,
	user: ArcSwapOption<user::Model>,
	delete: AtomicBool,
}

#[derive(Clone)]
pub struct Session {
	inner: Arc<SessionInner>,
}

#[async_trait]
impl<S> FromRequestParts<S> for Session {
	type Rejection = Infallible;

	async fn from_request_parts(
		parts: &mut request::Parts,
		_state: &S,
	) -> Result<Self, Self::Rejection> {
		parts.extensions.get().cloned().ok_or_else(|| {
			error!("session not found in request extensions");
			unreachable!()
		})
	}
}

impl Session {
	pub fn id(&self) -> Uuid {
		self.inner.id
	}

	pub fn user(&self) -> Option<Arc<user::Model>> {
		self.inner.user.load_full()
	}

	/// Note: this does not update the session in the database
	pub fn set_user(&self, user: Option<user::Model>) {
		self.inner.user.store(user.map(Arc::from))
	}

	pub fn delete(&self) {
		self.inner.delete.store(true, Ordering::Relaxed)
	}

	fn is_deleted(&self) -> bool {
		self.inner.delete.load(Ordering::Relaxed)
	}
}

#[derive(Clone)]
pub struct SessionLayer {
	key: Key,
	cookie: Arc<CookieConfig>,
	db: DatabaseConnection,
}

impl SessionLayer {
	pub fn new(master_key: &str, cookie: CookieConfig, db: DatabaseConnection) -> Self {
		Self {
			key: Key::derive_from(master_key.as_bytes()),
			cookie: Arc::new(cookie),
			db,
		}
	}
}

impl<S> Layer<S> for SessionLayer {
	type Service = SessionService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		SessionService {
			key: self.key.clone(),
			cookie: self.cookie.clone(),
			db: self.db.clone(),
			inner,
		}
	}
}

#[derive(Clone)]
pub struct SessionService<S> {
	key: Key,
	cookie: Arc<CookieConfig>,
	db: DatabaseConnection,
	inner: S,
}

const COOKIE_NAME: &str = "session";
const EXPIRY_USER: Duration = Duration::days(30);
const EXPIRY_ANONYMOUS: Duration = Duration::hours(12);
impl<S> SessionService<S> {
	async fn run<B>(mut self, mut req: Request<B>) -> Result<Response, Infallible>
	where
		S: Service<Request<B>, Response = Response> + Send + Clone + 'static,
		<S as Service<Request<B>>>::Error: Into<Infallible> + 'static,
		<S as Service<Request<B>>>::Future: Send,
		B: Send + 'static,
	{
		let store = CookieStore::new(req.headers(), self.key.clone());

		let session = match self.get_or_create_session(&store).await {
			Ok(v) => v,
			Err(e) => {
				error!("failed to get or create session: {e:#?}");
				return Ok((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response());
			}
		};

		let has_user = session.user().is_some();
		let span = info_span!("session", session.id = %session.id());
		req.extensions_mut().insert(session.clone());
		req.extensions_mut().insert(store.clone());
		let mut res = span
			.in_scope(|| self.inner.call(req))
			.instrument(span)
			.await
			.map_err(Into::into)?;

		let mut jar = store.into_jar();
		let has_user_after_request = session.user().is_some();

		if session.is_deleted() {
			jar.remove(
				Cookie::build(COOKIE_NAME, "")
					.path(self.cookie.path.clone())
					.finish(),
			);
		} else if !has_user && has_user_after_request {
			// the user was logged in during this request - update the session with
			// new expiry
			// TODO:
		}

		res.headers_mut().extend(
			jar
				.delta()
				.flat_map(|c| HeaderValue::from_str(&c.encoded().to_string()))
				.map(|value| (header::SET_COOKIE, value)),
		);

		Ok(res)
	}

	#[instrument(skip_all, err)]
	async fn create_session(
		&mut self,
		now: OffsetDateTime,
		jar: &CookieStore,
	) -> Result<session::Model, SessionError> {
		let model = session::ActiveModel {
			id: ActiveValue::NotSet,
			ctime: ActiveValue::Set(now.into_primitive_utc()),
			atime: ActiveValue::Set(now.into_primitive_utc()),
			etime: ActiveValue::Set((now + EXPIRY_ANONYMOUS).into_primitive_utc()),
			user_id: ActiveValue::Set(None),
		};

		let model = session::Entity::insert(model)
			.exec_with_returning(&self.db)
			.await?;

		debug!(session.id = %model.id, "created new session");
		let cookie = Cookie::build(COOKIE_NAME, model.id.to_string())
			.http_only(true)
			.apply(&self.cookie)
			.expires(model.etime.assume_utc())
			.same_site(SameSite::Strict)
			.finish();

		jar.add(cookie);
		Ok(model)
	}

	#[instrument(skip_all, err)]
	async fn update_session(
		&mut self,
		now: OffsetDateTime,
		model: session::Model,
		cookie: Cookie<'static>,
		jar: &CookieStore,
	) -> Result<session::Model, SessionError> {
		let expiry = if model.user_id.is_some() {
			EXPIRY_USER
		} else {
			EXPIRY_ANONYMOUS
		};

		let mut model: session::ActiveModel = model.into();
		model.atime = ActiveValue::Set(now.into_primitive_utc());
		model.etime = ActiveValue::Set((now + expiry).into_primitive_utc());

		let model = session::Entity::update(model).exec(&self.db).await?;

		debug!(session.id = %model.id, "updated session expiry");
		let mut new_cookie = cookie.clone().apply(&self.cookie);

		new_cookie.set_expires(model.etime.assume_utc());

		jar.add(new_cookie);
		Ok(model)
	}

	#[instrument(skip_all, err)]
	async fn get_or_create_session(&mut self, jar: &CookieStore) -> Result<Session, SessionError> {
		let now = OffsetDateTime::now_utc();
		let session = match jar.get(COOKIE_NAME) {
			None => {
				debug!("no session cookie");
				self.create_session(now, jar).await?
			}
			Some(cookie) => match Uuid::from_str(cookie.value()).ok() {
				None => {
					debug!(
						"session cookie ({value}) is not a valid uuid",
						value = cookie.value()
					);
					self.create_session(now, jar).await?
				}
				Some(id) => {
					match session::Entity::find_by_id(id).one(&self.db).await? {
						None => {
							debug!("session with id '{id}' not found in db");
							self.create_session(now, jar).await?
						}
						Some(model) => {
							// if it's been more than 1 hour since we last updated atime,
							// update it
							if model.atime.assume_utc() + Duration::hours(1) < now {
								self.update_session(now, model, cookie, jar).await?
							} else {
								debug!("session up to date");
								model
							}
						}
					}
				}
			},
		};

		let user = match session.user_id {
			None => None,
			Some(id) => user::Entity::find_by_id(id).one(&self.db).await?,
		};

		let session = Session {
			inner: Arc::new(SessionInner {
				id: session.id,
				user: ArcSwapOption::new(user.map(Arc::from)),
				delete: AtomicBool::new(false),
			}),
		};

		Ok(session)
	}
}

impl<B, S> Service<Request<B>> for SessionService<S>
where
	S: Service<Request<B>, Response = Response> + Send + Clone + 'static,
	<S as Service<Request<B>>>::Error: Into<Infallible> + 'static,
	<S as Service<Request<B>>>::Future: Send,
	B: Send + 'static,
{
	type Response = S::Response;
	type Error = Infallible;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(
		&mut self,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx).map_err(Into::into)
	}

	fn call(&mut self, req: Request<B>) -> Self::Future {
		self.clone().run(req).boxed()
	}
}

trait CookieBuilderExt {
	fn apply(self, config: &CookieConfig) -> Self;
}

impl<'a> CookieBuilderExt for CookieBuilder<'a> {
	fn apply(mut self, config: &CookieConfig) -> Self {
		self = self.secure(config.secure);
		self = self.path(config.path.clone());

		if let Some(domain) = config.domain.as_ref() {
			self = self.domain(Cow::Owned(domain.clone()));
		}

		self
	}
}

impl<'a> CookieBuilderExt for Cookie<'a> {
	fn apply(mut self, config: &CookieConfig) -> Self {
		self.set_secure(config.secure);
		self.set_path(config.path.clone());

		if let Some(domain) = config.domain.as_ref() {
			self.set_domain(Cow::Owned(domain.clone()));
		}

		self
	}
}
