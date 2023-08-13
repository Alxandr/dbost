use super::{AuthConfigBuilder, AuthProvider, AuthProviderName, CompleteAuthenticationError, User};
use async_trait::async_trait;
use indexmap::IndexSet;
use oauth2::{
	basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
	ClientSecret, CsrfToken, HttpRequest, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
	TokenResponse, TokenUrl,
};
use openidconnect::Nonce;
use serde::Deserialize;
use std::sync::Arc;
use tracing::error;
use url::Url;

const AUTHORIZATION_ENDPOINT: &str = "https://github.com/login/oauth/authorize";
const TOKEN_ENDPOINT: &str = "https://github.com/login/oauth/access_token";
const USERINFO_ENDPOINT: &str = "https://api.github.com/user";

#[derive(Debug, Clone)]
pub struct GithubAuthConfig {
	client_id: ClientId,
	client_secret: ClientSecret,
	redirect_uri: RedirectUrl,
	authorized_users: IndexSet<Box<str>>,
}

impl GithubAuthConfig {
	pub fn new(
		client_id: impl Into<String>,
		client_secret: impl Into<String>,
		redirect_uri: impl Into<Url>,
		authorized_users: impl IntoIterator<Item = impl Into<Box<str>>>,
	) -> Self {
		Self {
			client_id: ClientId::new(client_id.into()),
			client_secret: ClientSecret::new(client_secret.into()),
			redirect_uri: RedirectUrl::from_url(redirect_uri.into()),
			authorized_users: authorized_users.into_iter().map(Into::into).collect(),
		}
	}
}

#[derive(Debug)]
struct GithubAuth {
	client: BasicClient,
	authorized_users: IndexSet<Box<str>>,
}

impl From<GithubAuthConfig> for GithubAuth {
	fn from(value: GithubAuthConfig) -> Self {
		let client = BasicClient::new(
			value.client_id,
			Some(value.client_secret),
			AuthUrl::new(AUTHORIZATION_ENDPOINT.into()).unwrap(),
			Some(TokenUrl::new(TOKEN_ENDPOINT.into()).unwrap()),
		)
		.set_redirect_uri(value.redirect_uri);

		Self {
			client,
			authorized_users: value.authorized_users,
		}
	}
}

#[derive(Debug, Clone)]
pub struct GithubAuthProvider {
	config: Arc<GithubAuth>,
}

impl AuthProviderName for GithubAuthProvider {
	const NAME: &'static str = "github";
}

#[async_trait]
impl AuthProvider for GithubAuthProvider {
	fn name(&self) -> &'static str {
		Self::NAME
	}

	fn authenticate(&self, state: CsrfToken, _nonce: Nonce, challenge: PkceCodeChallenge) -> Url {
		// GitHub doens't use OIDC, and so doesn't take nonce
		self
			.config
			.client
			.authorize_url(|| state)
			.add_scope(Scope::new(String::from("user")))
			.add_scope(Scope::new(String::from("user:email")))
			.set_pkce_challenge(challenge)
			.url()
			.0
	}

	async fn callback(
		&self,
		code: AuthorizationCode,
		verifier: PkceCodeVerifier,
		_nonce: Nonce,
	) -> Result<User, CompleteAuthenticationError> {
		let response = match self
			.config
			.client
			.exchange_code(code)
			.set_pkce_verifier(verifier)
			.request_async(async_http_client)
			.await
		{
			Ok(res) => res,
			Err(e) => {
				error!("Failed to exchange code: {}", e);
				return Err(CompleteAuthenticationError::other(e));
			}
		};

		let access_token = response.access_token();

		let mut headers = http::header::HeaderMap::new();
		headers.insert(
			http::header::USER_AGENT,
			http::HeaderValue::from_static("dbost"),
		);
		headers.insert(
			http::header::ACCEPT,
			http::HeaderValue::from_static("application/vnd.github+json"),
		);
		headers.insert(
			http::header::HeaderName::from_static("x-github-api-version"),
			http::HeaderValue::from_static("2022-11-28"),
		);
		headers.insert(
			http::header::AUTHORIZATION,
			http::header::HeaderValue::from_str(&format!("Bearer {}", access_token.secret())).unwrap(),
		);

		let user_info_request = HttpRequest {
			url: Url::parse(USERINFO_ENDPOINT).unwrap(),
			method: http::Method::GET,
			headers,
			body: vec![],
		};

		let user_info_response = match async_http_client(user_info_request).await {
			Err(e) => {
				error!("Failed to get user info: {}", e);
				return Err(CompleteAuthenticationError::other(e));
			}
			Ok(res) if !res.status_code.is_success() => {
				// let _ = dbg!(std::str::from_utf8(&res.body));
				error!("Failed to get user info: {}", res.status_code);
				return Err(CompleteAuthenticationError::other(
					"failed to get user info",
				));
			}
			Ok(res) => res,
		};

		let user: GitHubUser = match serde_json::from_slice(&user_info_response.body) {
			Ok(user) => user,
			Err(e) => {
				error!("Failed to parse user info: {}", e);
				return Err(CompleteAuthenticationError::other(e));
			}
		};

		if !self.config.authorized_users.contains(&*user.login) {
			return Err(CompleteAuthenticationError::InvalidUser);
		}

		Ok(User {
			display_name: user.name.unwrap_or_else(|| user.login.clone()),
			id: user.login,
			email: user.email,
			avatar_url: user.avatar_url,
		})
	}
}

impl AuthConfigBuilder {
	pub fn with_github(self, config: GithubAuthConfig) -> Self {
		self.with_provider(GithubAuthProvider {
			config: Arc::new(GithubAuth::from(config)),
		})
	}
}

#[derive(Deserialize)]
struct GitHubUser {
	login: String,
	email: String,
	#[serde(default = "Default::default")]
	name: Option<String>,
	#[serde(default = "Default::default")]
	avatar_url: Option<Url>,
}
