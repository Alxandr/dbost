use async_trait::async_trait;
use axum::extract::FromRequestParts;
use cookie::{Cookie, CookieJar, Key, PrivateJar};
use http::{header, request::Parts, HeaderMap};
use std::{
	convert::Infallible,
	sync::{Arc, Mutex},
};

#[derive(Clone)]
struct Inner {
	key: Key,
	jar: CookieJar,
}

#[derive(Clone)]
pub struct CookieStore(Arc<Mutex<Inner>>);

impl CookieStore {
	pub(crate) fn new(headers: &HeaderMap, key: Key) -> Self {
		let mut jar = CookieJar::new();

		headers
			.get_all(header::COOKIE)
			.into_iter()
			.flat_map(|header| header.to_str())
			.flat_map(Cookie::split_parse_encoded)
			.flatten()
			.map(Cookie::into_owned)
			.for_each(|c| jar.add_original(c));

		let inner = Inner { jar, key };

		Self(Arc::new(Mutex::new(inner)))
	}

	pub(crate) fn into_jar(self) -> CookieJar {
		Arc::try_unwrap(self.0)
			.map(|mutex| mutex.into_inner().unwrap().jar)
			.unwrap_or_else(|arc| arc.lock().unwrap().jar.clone())
	}

	fn with_jar<T>(&self, f: impl FnOnce(PrivateJar<&mut CookieJar>) -> T) -> T {
		let mut guard = self.0.lock().unwrap();
		let inner = &mut *guard;
		let key = &inner.key;
		let jar = &mut inner.jar;
		let private = jar.private_mut(key);
		f(private)
	}

	pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
		self.with_jar(|jar| jar.get(name))
	}

	pub fn add(&self, cookie: Cookie<'static>) {
		self.with_jar(|mut jar| jar.add(cookie));
	}

	pub fn remove(&self, cookie: Cookie<'static>) {
		self.with_jar(|mut jar| jar.remove(cookie));
	}
}

#[async_trait]
impl<S> FromRequestParts<S> for CookieStore {
	type Rejection = Infallible;

	/// Perform the extraction.
	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
		let store: &Self = parts
			.extensions
			.get()
			.expect("missing cookie store, did you forget session layer");

		Ok(store.clone())
	}
}
