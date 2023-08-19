use axum::{body::Body, Router};
use tower_livereload::LiveReloadLayer;

#[derive(Clone, Copy)]
struct NotHxRequestPredicate;

impl tower_livereload::predicate::Predicate<axum::http::Request<axum::body::Body>>
	for NotHxRequestPredicate
{
	fn check(&mut self, p: &axum::http::Request<axum::body::Body>) -> bool {
		!p.headers()
			.contains_key(&dbost_htmx::headers::request::HX_REQUEST)
	}
}

pub fn configure<S: Clone + Send + Sync + 'static>(router: Router<S, Body>) -> Router<S, Body> {
	router.layer(LiveReloadLayer::new().request_predicate(NotHxRequestPredicate))
}
