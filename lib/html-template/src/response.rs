use crate::HtmlTemplate;
use axum::{
	headers::ContentType,
	http::StatusCode,
	response::{IntoResponse, Response},
	TypedHeader,
};

pub struct HtmlTemplateResponse<T: HtmlTemplate> {
	template: T,
}

impl<T: HtmlTemplate> HtmlTemplateResponse<T> {
	pub fn new(template: T) -> Self {
		Self { template }
	}
}

impl<T: HtmlTemplate> IntoResponse for HtmlTemplateResponse<T> {
	fn into_response(self) -> Response {
		match self.template.into_bytes() {
			Ok(bytes) => (TypedHeader(ContentType::html()), bytes).into_response(),

			Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response(),
		}
	}
}

pub trait HtmlTemplateResponseExt {
	fn into_response(self) -> Response;
}

impl<T: HtmlTemplate> HtmlTemplateResponseExt for T {
	fn into_response(self) -> Response {
		HtmlTemplateResponse::new(self).into_response()
	}
}
