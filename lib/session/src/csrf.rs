use crate::{Cookie, CookieStore, Session};
use hmac::{Hmac, Mac};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::OnceLock;
use thiserror::Error;
use uuid::Uuid;

const COOKIE_NAME: &str = ".csrf";

type HmacSha256 = Hmac<Sha256>;

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

#[derive(Serialize, Deserialize, Debug)]
struct CsrfToken {
	session_id: Uuid,
	nonce: [u8; 32],
}

#[derive(Debug, Error)]
pub enum CsrfError {
	#[error(transparent)]
	Serialization(#[from] bincode::Error),

	#[error(transparent)]
	HmacInvalidLengthKey(#[from] crypto_common::InvalidLength),

	#[error(transparent)]
	InvalidSignature(#[from] digest::MacError),

	#[error(transparent)]
	InvalidBase64(#[from] base64::DecodeError),
}

impl CsrfToken {
	fn to_sealed_string(&self, key: &[u8]) -> Result<String, CsrfError> {
		let mut mac = HmacSha256::new_from_slice(key)?;
		let mut message = bincode::serialize(self)?;
		mac.update(&message);

		let result = mac.finalize().into_bytes();
		message.extend(result);
		Ok(base64_encode(&message))
	}

	fn from_sealed_string(key: &[u8], sealed: &str) -> Result<Self, CsrfError> {
		let message = base64_decode(sealed)?;
		let (message, signature) = message.split_at(message.len() - 32);

		// validate signature
		let mut mac = HmacSha256::new_from_slice(key)?;
		mac.update(message);
		mac.verify_slice(signature)?;

		let token: Self = bincode::deserialize(message)?;
		Ok(token)
	}
}

struct CsrfConfig<'a> {
	secure: bool,
	key: &'a [u8],
}

fn create_csrf_token(
	config: CsrfConfig,
	session: &Session,
	store: &CookieStore,
) -> Result<super::CsrfToken, CsrfError> {
	let token = CsrfToken {
		session_id: session.id(),
		nonce: rand::thread_rng().gen(),
	};

	let sealed = token.to_sealed_string(config.key)?;
	let cookie = Cookie::build(COOKIE_NAME, sealed.clone())
		.http_only(true)
		.same_site(cookie::SameSite::Strict)
		.secure(config.secure)
		.path("/")
		.finish();

	store.add(cookie);
	Ok(crate::CsrfToken {
		inner: sealed.into(),
	})
}

pub(crate) fn get_or_create_csrf_token(
	session: &Session,
	store: &CookieStore,
	key: &[u8],
	secure: bool,
) -> Result<crate::CsrfToken, CsrfError> {
	let config = CsrfConfig { key, secure };

	match store.get(COOKIE_NAME) {
		None => create_csrf_token(config, session, store),
		Some(cookie) => {
			let value = cookie.value();
			let token = CsrfToken::from_sealed_string(config.key, value)?;
			if token.session_id == session.id() {
				Ok(crate::CsrfToken {
					inner: value.into(),
				})
			} else {
				create_csrf_token(config, session, store)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	const TEST_KEY: &[u8] = b"foo bar baz abc def long ass key goes here";

	#[test]
	fn csrf_token_roundtrip() {
		let token = CsrfToken {
			session_id: Uuid::new_v4(),
			nonce: rand::thread_rng().gen(),
		};

		let sealed = token.to_sealed_string(TEST_KEY).expect("sealing works");
		let unsealed = CsrfToken::from_sealed_string(TEST_KEY, &sealed).expect("unsealing works");

		assert_eq!(token.nonce, unsealed.nonce);
		assert_eq!(token.session_id, unsealed.session_id);
	}
}
