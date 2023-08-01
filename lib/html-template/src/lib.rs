use axum::{
	body,
	http::{header, HeaderMap, HeaderValue},
	response::{IntoResponse, Response},
};
use bytes::{BufMut, Bytes, BytesMut};
use futures::future::FusedFuture;
use http_body::Body;
use pin_project::pin_project;
use std::{
	convert::Infallible,
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

pub use html_template_macro::template;

#[macro_export]
macro_rules! response_html {
	($($tt:tt)*) => {
		$crate::HtmlTemplateExt::into_response($crate::template!($($tt)*))
	};
}

pub trait HtmlFormatter {
	type Error: std::error::Error;

	fn write_raw(&mut self, raw: &str) -> Result<(), Self::Error>;

	fn write_doctype(&mut self, value: &str) -> Result<(), Self::Error> {
		self.write_raw("<!DOCTYPE ")?;
		self.write_raw(value)?;
		self.write_raw(">")
	}

	fn write_open_tag_start(&mut self, tag: &str) -> Result<(), Self::Error> {
		// TODO: validate tag name?
		self.write_raw("<")?;
		self.write_raw(tag)
	}

	fn write_attribute_name(&mut self, name: &str) -> Result<(), Self::Error> {
		// TODO: validate attribute name?
		self.write_raw(" ")?;
		self.write_raw(name)
	}

	fn write_attribute_value(&mut self, value: &str) -> Result<(), Self::Error> {
		self.write_raw("=\"")?;
		// TODO: escape attribute value
		self.write_raw(value)?;
		self.write_raw("\"")
	}

	fn write_open_tag_end(&mut self, self_closing: bool) -> Result<(), Self::Error> {
		if self_closing {
			self.write_raw(" />")
		} else {
			self.write_raw(">")
		}
	}

	fn write_end_tag(&mut self, tag: &str) -> Result<(), Self::Error> {
		self.write_raw("</")?;
		// TODO: validate tag name?
		self.write_raw(tag)?;
		self.write_raw(">")
	}

	fn write_text(&mut self, text: &str) -> Result<(), Self::Error> {
		// TODO: escape text
		self.write_raw(text)
	}

	fn write_comment(&mut self, comment: &str) -> Result<(), Self::Error> {
		self.write_raw("<!--")?;
		// TODO: escape comment
		self.write_raw(comment)?;
		self.write_raw("-->")
	}
}

pub trait HtmlTemplate {
	fn fmt<F>(self, formatter: &mut F) -> Result<(), F::Error>
	where
		F: HtmlFormatter;
}

pub trait HtmlAttributeValue {
	fn fmt<F>(self, formatter: &mut F) -> Result<(), F::Error>
	where
		F: HtmlFormatter;
}

impl HtmlAttributeValue for String {
	fn fmt<F>(self, formatter: &mut F) -> Result<(), F::Error>
	where
		F: HtmlFormatter,
	{
		formatter.write_attribute_value(&self)
	}
}

impl<'a> HtmlAttributeValue for &'a str {
	fn fmt<F>(self, formatter: &mut F) -> Result<(), F::Error>
	where
		F: HtmlFormatter,
	{
		formatter.write_attribute_value(self)
	}
}

impl HtmlTemplate for String {
	fn fmt<F>(self, formatter: &mut F) -> Result<(), F::Error>
	where
		F: HtmlFormatter,
	{
		formatter.write_text(&self)
	}
}

impl<'a> HtmlTemplate for &'a str {
	fn fmt<F>(self, formatter: &mut F) -> Result<(), F::Error>
	where
		F: HtmlFormatter,
	{
		formatter.write_text(self)
	}
}

pub struct HtmlIterTemplate<I>(I);

impl<I> HtmlTemplate for HtmlIterTemplate<I>
where
	I: IntoIterator,
	I::Item: HtmlTemplate,
{
	fn fmt<F>(self, formatter: &mut F) -> Result<(), F::Error>
	where
		F: HtmlFormatter,
	{
		for item in self.0 {
			item.fmt(formatter)?;
		}
		Ok(())
	}
}

pub trait HtmlTemplateIterExtensions: IntoIterator + Sized {
	fn into_template(self) -> HtmlIterTemplate<Self>;
}

impl<I> HtmlTemplateIterExtensions for I
where
	I: IntoIterator,
	I::Item: HtmlTemplate,
{
	fn into_template(self) -> HtmlIterTemplate<Self> {
		HtmlIterTemplate(self)
	}
}

pub struct HtmlWriter<'a, W> {
	writer: &'a mut W,
}

impl<'a, W> HtmlWriter<'a, W> {
	pub fn new(writer: &'a mut W) -> Self {
		Self { writer }
	}
}

impl<'a, W: BufMut> HtmlFormatter for HtmlWriter<'a, W> {
	type Error = Infallible;

	fn write_raw(&mut self, raw: &str) -> Result<(), Self::Error> {
		self.writer.put_slice(raw.as_bytes());
		Ok(())
	}

	fn write_comment(&mut self, _comment: &str) -> Result<(), Self::Error> {
		// don't write comments in production
		Ok(())
	}
}

#[pin_project]
struct HtmlTemplateBody<T: HtmlTemplate>(Option<T>);

impl<T: HtmlTemplate> Future for HtmlTemplateBody<T> {
	type Output = Option<Result<Bytes, Infallible>>;

	fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
		Poll::Ready(self.project().0.take().map(|template| {
			let mut bytes = BytesMut::new();
			let mut writer = HtmlWriter::new(&mut bytes);
			template.fmt(&mut writer).unwrap();
			Ok(bytes.freeze())
		}))
	}
}

impl<T: HtmlTemplate> FusedFuture for HtmlTemplateBody<T> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

impl<T: HtmlTemplate> Body for HtmlTemplateBody<T> {
	type Data = Bytes;
	type Error = Infallible;

	fn poll_data(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Self::Data, Self::Error>>> {
		self.poll(cx)
	}

	fn poll_trailers(
		self: Pin<&mut Self>,
		_cx: &mut Context<'_>,
	) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
		Poll::Ready(Ok(None))
	}
}

impl<T: HtmlTemplate + Send + 'static> IntoResponse for HtmlTemplateBody<T> {
	fn into_response(self) -> Response {
		Response::new(body::boxed(self))
	}
}

pub trait HtmlTemplateExt: HtmlTemplate + Send + 'static {
	fn into_response(self) -> Response;
}

impl<T: HtmlTemplate + Send + Sized + 'static> HtmlTemplateExt for T {
	fn into_response(self) -> Response {
		let mut response = Response::new(body::boxed(HtmlTemplateBody(Some(self))));
		response.headers_mut().insert(
			header::CONTENT_TYPE,
			HeaderValue::from_static("text/html; charset=utf-8"),
		);
		response
	}
}
