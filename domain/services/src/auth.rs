mod github;

use async_trait::async_trait;
use axum::{
	extract::{FromRef, FromRequestParts},
	response::Redirect,
};
use cookie::{Cookie, SameSite};
use dbost_entities::{session, user, user_link};
use dbost_session::{CookieStore, Session};
use futures::FutureExt;
use indexmap::IndexMap;
use oauth2::{AuthorizationCode, PkceCodeVerifier};
use openidconnect::{CsrfToken, Nonce, PkceCodeChallenge};
use sea_orm::{
	ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DatabaseTransaction, DbErr,
	EntityTrait, IsolationLevel, QueryFilter, TransactionError, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::{
	borrow::Borrow,
	convert::Infallible,
	fmt,
	sync::{Arc, OnceLock},
};
use thiserror::Error;
use time::Duration;
use tracing::error;
use url::Url;

pub use github::GithubAuthConfig;

fn base64_engine() -> &'static impl base64::Engine {
	static ENGINE: OnceLock<base64::engine::GeneralPurpose> = OnceLock::new();

	ENGINE.get_or_init(|| {
		let config = base64::engine::GeneralPurposeConfig::new()
			.with_encode_padding(false)
			.with_decode_padding_mode(base64::engine::DecodePaddingMode::RequireNone);

		base64::engine::GeneralPurpose::new(&base64::alphabet::URL_SAFE, config)
	})
}

fn base64_encode(data: &[u8]) -> String {
	use base64::Engine;
	base64_engine().encode(data)
}

fn base64_decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
	use base64::Engine;
	base64_engine().decode(data)
}

#[derive(Default, Debug)]
struct AuthServiceConfig {
	db: DatabaseConnection,
	secure: bool,
	base_path: String,
	providers: IndexMap<&'static str, Box<dyn AuthProvider + Send + Sync + 'static>>,
}

#[derive(Clone)]
pub struct AuthConfig {
	config: Arc<AuthServiceConfig>,
}

impl AuthConfig {
	pub fn builder(db: DatabaseConnection) -> AuthConfigBuilder {
		AuthConfigBuilder {
			config: AuthServiceConfig {
				db,
				secure: true,
				base_path: "/".into(),
				providers: IndexMap::new(),
			},
		}
	}

	fn get(&self, name: &str) -> Option<&(dyn AuthProvider + Send + Sync + 'static)> {
		match self.config.providers.get(name) {
			None => None,
			Some(b) => Some(b.borrow()),
		}
	}

	fn base_path(&self) -> &str {
		&self.config.base_path
	}

	fn secure(&self) -> bool {
		self.config.secure
	}

	fn db(&self) -> &DatabaseConnection {
		&self.config.db
	}
}

pub struct AuthConfigBuilder {
	config: AuthServiceConfig,
}

impl From<AuthConfigBuilder> for AuthConfig {
	fn from(value: AuthConfigBuilder) -> Self {
		Self {
			config: Arc::new(value.config),
		}
	}
}

impl AuthConfigBuilder {
	pub fn build(self) -> AuthConfig {
		self.into()
	}

	pub fn secure_cookies(mut self, secure: bool) -> Self {
		self.config.secure = secure;
		self
	}

	pub fn base_path(mut self, base_path: impl Into<String>) -> Self {
		self.config.base_path = base_path.into();
		self
	}

	pub fn with_provider<P>(mut self, provider: P) -> Self
	where
		P: AuthProvider + AuthProviderName + Send + Sync + 'static,
	{
		self.config.providers.insert(P::NAME, Box::new(provider));

		self
	}
}

#[derive(Clone)]
pub struct AuthService {
	config: AuthConfig,
	cookies: CookieStore,
	session: Session,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthService
where
	AuthConfig: FromRef<S>,
	S: Sync,
{
	type Rejection = Infallible;

	async fn from_request_parts(
		parts: &mut http::request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		let config = AuthConfig::from_ref(state);
		let cookies = parts
			.extensions
			.get::<CookieStore>()
			.expect("CookieStore not found")
			.clone();
		let session = parts
			.extensions
			.get::<Session>()
			.expect("Session not found")
			.clone();

		Ok(Self {
			config,
			cookies,
			session,
		})
	}
}

#[derive(Debug, Error)]
pub enum StartAuthenticationError {
	#[error("invalid provider: {0}")]
	InvalidProvider(String),
}

#[derive(Debug, Error)]
pub enum CompleteAuthenticationError {
	#[error("invalid provider: {0}")]
	InvalidProvider(String),

	#[error("login window closed")]
	LoginWindowClosed,

	#[error("invalid user")]
	InvalidUser,

	#[error("user not found")]
	UserNotFound,

	#[error("email in use")]
	EmailInUse,

	#[error("other error: {0}")]
	Other(String),
}

impl CompleteAuthenticationError {
	fn other(error: impl fmt::Display) -> Self {
		Self::Other(error.to_string())
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthenticationFlowState {
	verifier: PkceCodeVerifier,
	nonce: Nonce,
	return_to: String,
	context: AuthenticationContext,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum AuthenticationContext {
	#[serde(rename = "login")]
	Login,
	#[serde(rename = "register")]
	Register,
}

pub struct User {
	pub id: String,
	pub display_name: String,
	pub email: String,
	pub avatar_url: Option<Url>,
}

pub trait AuthProviderName {
	const NAME: &'static str;
}

#[async_trait]
pub trait AuthProvider: fmt::Debug {
	fn name(&self) -> &'static str;

	fn authenticate(&self, state: CsrfToken, nonce: Nonce, challenge: PkceCodeChallenge) -> Url;

	async fn callback(
		&self,
		code: AuthorizationCode,
		verifier: PkceCodeVerifier,
		nonce: Nonce,
	) -> Result<User, CompleteAuthenticationError>;
}

impl AuthService {
	fn cookie_name(state: &CsrfToken) -> String {
		format!(".auth.{state}", state = state.secret())
	}

	pub async fn logout(&self) -> Result<Redirect, DbErr> {
		self.update_session_user(None).await?;

		Ok(Redirect::to("/"))
	}

	pub fn login(
		&self,
		provider: &str,
		return_to: &str,
	) -> Result<Redirect, StartAuthenticationError> {
		self.authenticate(provider, return_to, AuthenticationContext::Login)
	}

	pub fn register(
		&self,
		provider: &str,
		return_to: &str,
	) -> Result<Redirect, StartAuthenticationError> {
		self.authenticate(provider, return_to, AuthenticationContext::Register)
	}

	fn authenticate(
		&self,
		provider: &str,
		return_to: &str,
		context: AuthenticationContext,
	) -> Result<Redirect, StartAuthenticationError> {
		let provider = self
			.config
			.get(provider)
			.ok_or_else(|| StartAuthenticationError::InvalidProvider(provider.to_string()))?;

		let state = CsrfToken::new_random();
		let nonce = Nonce::new_random();
		let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

		let cookie_state = AuthenticationFlowState {
			verifier,
			nonce: nonce.clone(),
			return_to: return_to.to_string(),
			context,
		};

		let cookie_value = bincode::serialize(&cookie_state).unwrap();
		let cookie_value = base64_encode(&cookie_value);
		let cookie = Cookie::build(Self::cookie_name(&state), cookie_value)
			.http_only(true)
			.path(self.config.base_path().to_owned())
			.secure(self.config.secure())
			.same_site(SameSite::Lax)
			.max_age(Duration::minutes(15))
			.finish();

		let url = provider.authenticate(state, nonce, challenge);
		self.cookies.add(cookie);

		Ok(Redirect::to(url.as_str()))
	}

	pub async fn callback(
		&self,
		provider: &str,
		code: &str,
		state: &str,
	) -> Result<Redirect, CompleteAuthenticationError> {
		let provider = self
			.config
			.get(provider)
			.ok_or_else(|| CompleteAuthenticationError::InvalidProvider(provider.to_string()))?;

		let cookie_name = Self::cookie_name(&CsrfToken::new(state.to_string()));
		let cookie = self
			.cookies
			.get(&cookie_name)
			.ok_or(CompleteAuthenticationError::LoginWindowClosed)?;

		let cookie_value = base64_decode(cookie.value()).map_err(CompleteAuthenticationError::other)?;

		let cookie_state: AuthenticationFlowState =
			bincode::deserialize(&cookie_value).map_err(CompleteAuthenticationError::other)?;

		self.cookies.remove(
			Cookie::build(cookie_name, "")
				.path(self.config.config.base_path.clone())
				.finish(),
		);
		let user = provider
			.callback(
				AuthorizationCode::new(code.into()),
				cookie_state.verifier,
				cookie_state.nonce,
			)
			.await?;

		match cookie_state.context {
			AuthenticationContext::Login => {
				self
					.db_login(provider.name(), user, &cookie_state.return_to)
					.await
			}
			AuthenticationContext::Register => {
				self
					.db_register(provider.name(), user, &cookie_state.return_to)
					.await
			}
		}
	}

	async fn get_user(
		&self,
		provider: &str,
		user: &User,
	) -> Result<Option<user::Model>, CompleteAuthenticationError> {
		let db = self.config.db();

		let link = user_link::Entity::find()
			.filter(user_link::Column::Service.eq(provider))
			.filter(user_link::Column::ServiceUserid.eq(user.id.as_str()))
			.one(db)
			.await
			.map_err(CompleteAuthenticationError::other)?;

		match link {
			None => Ok(None),
			Some(link) => user::Entity::find_by_id(link.user_id)
				.one(db)
				.await
				.map_err(CompleteAuthenticationError::other)?
				.ok_or_else(|| {
					error!("user not found in db based on link id - this should not happen due to database constraint");
					CompleteAuthenticationError::other("user not found in db based on link id")
				})
				.map(Some),
		}
	}

	async fn update_session_user(&self, user: Option<user::Model>) -> Result<(), DbErr> {
		match &user {
			None => {
				session::Entity::delete_by_id(self.session.id())
					.exec(self.config.db())
					.await?;
			}

			Some(user) => {
				let session_update = session::ActiveModel {
					id: ActiveValue::Unchanged(self.session.id()),
					user_id: ActiveValue::Set(Some(user.id)),
					..Default::default()
				};

				session::Entity::update(session_update)
					.exec(self.config.db())
					.await?;
			}
		}

		self.session.set_user(user);
		Ok(())
	}

	async fn db_login(
		&self,
		provider: &'static str,
		user: User,
		return_to: &str,
	) -> Result<Redirect, CompleteAuthenticationError> {
		let user = self
			.get_user(provider, &user)
			.await?
			.ok_or(CompleteAuthenticationError::UserNotFound)?;

		self
			.update_session_user(Some(user))
			.await
			.map_err(CompleteAuthenticationError::other)?;
		Ok(Redirect::to(return_to))
	}

	async fn db_register(
		&self,
		provider: &'static str,
		user: User,
		return_to: &str,
	) -> Result<Redirect, CompleteAuthenticationError> {
		async fn register_user(
			tx: &DatabaseTransaction,
			provider: &str,
			user: User,
		) -> Result<user::Model, CompleteAuthenticationError> {
			// first, check if there is an existing user with the same email
			let existing_user = user::Entity::find()
				.filter(user::Column::Email.eq(user.email.as_str()))
				.one(tx)
				.await
				.map_err(CompleteAuthenticationError::other)?;

			if existing_user.is_some() {
				return Err(CompleteAuthenticationError::EmailInUse);
			}

			// create the user
			let new_user = user::ActiveModel {
				id: ActiveValue::NotSet,
				display_name: ActiveValue::Set(user.display_name),
				email: ActiveValue::Set(user.email),
				avatar_url: ActiveValue::Set(user.avatar_url.map(|u| u.to_string())),
			};

			let new_user = new_user
				.insert(tx)
				.await
				.map_err(CompleteAuthenticationError::other)?;

			// create the link
			let link = user_link::ActiveModel {
				user_id: ActiveValue::Set(new_user.id),
				service: ActiveValue::Set(provider.into()),
				service_userid: ActiveValue::Set(user.id.clone()),
			};

			link
				.insert(tx)
				.await
				.map_err(CompleteAuthenticationError::other)?;

			Ok(new_user)
		}

		// first - we check if we already have a user. If we do, we can simply just use it
		if let Some(user) = self.get_user(provider, &user).await? {
			self
				.update_session_user(Some(user))
				.await
				.map_err(CompleteAuthenticationError::other)?;
			return Ok(Redirect::to(return_to));
		};

		// run the rest of the logic in a transaction
		let new_user = self
			.config
			.db()
			.transaction_with_config(
				|tx| register_user(tx, provider, user).boxed(),
				Some(IsolationLevel::Serializable),
				None,
			)
			.await
			.map_err(|e| match e {
				TransactionError::Connection(e) => CompleteAuthenticationError::other(e),
				TransactionError::Transaction(e) => e,
			})?;

		self
			.update_session_user(Some(new_user))
			.await
			.map_err(CompleteAuthenticationError::other)?;
		Ok(Redirect::to(return_to))
	}
}
