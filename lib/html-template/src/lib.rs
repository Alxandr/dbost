use bytes::{BufMut, Bytes, BytesMut};
use std::fmt;

mod escape;
mod response;

pub use html_template_macro::component;
pub use response::{HtmlTemplateResponse, HtmlTemplateResponseExt};

pub struct HtmlAttributeFormatter<'a> {
	any_written: bool,
	buffer: &'a mut BytesMut,
}

impl<'a> HtmlAttributeFormatter<'a> {
	fn new(buffer: &'a mut BytesMut) -> Self {
		Self {
			any_written: false,
			buffer,
		}
	}

	pub fn write_bytes(&mut self, raw: &[u8]) {
		if !self.any_written {
			self.any_written = true;
			self.buffer.put_slice(b"=\"");
		}

		self.buffer.put_slice(raw);
	}

	pub fn write(&mut self, value: &[u8]) {
		self.write_bytes(&escape::attribute(value))
	}
}

pub struct HtmlFormatter<'a> {
	buffer: &'a mut BytesMut,
}

impl<'a> HtmlFormatter<'a> {
	pub fn new(buffer: &'a mut BytesMut) -> Self {
		Self { buffer }
	}

	pub fn write_bytes(&mut self, raw: &[u8]) {
		self.buffer.put_slice(raw);
	}

	pub fn write(&mut self, value: &[u8]) {
		self.write_bytes(&escape::text(value))
	}

	pub fn write_doctype(&mut self, value: &[u8]) {
		self.write_bytes(b"<!DOCTYPE ");
		self.write_bytes(&escape::text(value));
		self.write_bytes(b">");
	}

	pub fn write_open_tag_start(&mut self, tag: &[u8]) {
		// TODO: validate tag name?
		self.write_bytes(b"<");
		self.write_bytes(tag);
	}

	pub fn write_attribute_name(&mut self, name: &[u8]) {
		// TODO: validate attribute name?
		self.write_bytes(b" ");
		self.write_bytes(name);
	}

	pub fn write_attribute_value(&mut self, value: impl HtmlAttributeValue) -> fmt::Result {
		let mut attribute_formatter = HtmlAttributeFormatter::new(self.buffer);

		value.fmt(&mut attribute_formatter)?;
		if attribute_formatter.any_written {
			self.write_bytes(b"\"");
		}

		Ok(())
	}

	pub fn write_self_close_tag(&mut self) {
		self.write_bytes(b" />");
	}

	pub fn write_open_tag_end(&mut self) {
		self.write_bytes(b">");
	}

	pub fn write_end_tag(&mut self, tag: &[u8]) {
		self.write_bytes(b"</");
		self.write_bytes(tag);
		self.write_bytes(b">");
	}

	pub fn write_content(&mut self, content: impl HtmlTemplate) -> fmt::Result {
		content.fmt(self)
	}

	pub fn write_comment(&mut self, comment: &[u8]) {
		self.write_bytes(b"<!--");
		self.write_bytes(&escape::text(comment));
		self.write_bytes(b"-->");
	}
}

pub trait HtmlComponent {
	type Template: HtmlTemplate;

	fn into_template(self) -> Self::Template;
}

// /// Marker trait for types that can be used as values in HTML templates
// /// by reference.
// pub trait ByRefValue {}

pub trait HtmlTemplate: Sized {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result;

	fn into_bytes(self) -> Result<Bytes, fmt::Error> {
		let mut buffer = BytesMut::new();
		let mut formatter = HtmlFormatter::new(&mut buffer);
		self.fmt(&mut formatter)?;
		Ok(buffer.freeze())
	}

	fn into_string(self) -> Result<String, fmt::Error> {
		let bytes = self.into_bytes()?;
		String::from_utf8(bytes.to_vec()).map_err(|_| fmt::Error)
	}
}

pub trait HtmlAttributeValue {
	fn fmt(self, formatter: &mut HtmlAttributeFormatter) -> fmt::Result;
}

// impl<T: HtmlTemplate + ByRefValue> HtmlTemplate for &T {
// 	fn fmt<F: ?Sized>(&self, formatter: &mut F) -> Result<(), F::Error>
// 	where
// 		F: HtmlFormatter,
// 	{
// 		(*self).fmt(formatter)
// 	}
// }

// impl<T: HtmlAttributeValue + ByRefValue> HtmlAttributeValue for &T {
// 	fn fmt<F: ?Sized>(&self, formatter: &mut F) -> Result<(), F::Error>
// 	where
// 		F: HtmlAttributeFormatter,
// 	{
// 		(*self).fmt(formatter)
// 	}
// }

impl HtmlTemplate for () {
	fn fmt(self, _formatter: &mut HtmlFormatter) -> fmt::Result {
		Ok(())
	}
}

impl HtmlAttributeValue for () {
	fn fmt(self, _formatter: &mut HtmlAttributeFormatter) -> fmt::Result {
		Ok(())
	}
}

impl<T: HtmlTemplate> HtmlTemplate for Option<T> {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		match self {
			None => Ok(()),
			Some(template) => template.fmt(formatter),
		}
	}
}

impl<T: HtmlAttributeValue> HtmlAttributeValue for Option<T> {
	fn fmt(self, formatter: &mut HtmlAttributeFormatter) -> fmt::Result {
		match self {
			None => Ok(()),
			Some(template) => template.fmt(formatter),
		}
	}
}

fn display(value: fmt::Arguments, mut write: impl FnMut(&[u8])) -> fmt::Result {
	match value.as_str() {
		Some(s) => {
			write(s.as_bytes());
			Ok(())
		}
		None => {
			use fmt::Write;
			struct Writer<F> {
				writer: F,
			}

			impl<F> Write for Writer<F>
			where
				F: FnMut(&[u8]),
			{
				fn write_str(&mut self, s: &str) -> fmt::Result {
					(self.writer)(s.as_bytes());
					Ok(())
				}
			}

			let mut writer = Writer { writer: &mut write };

			write!(&mut writer, "{}", value)
		}
	}
}

macro_rules! impl_simple_write {
	($ty:ty, as_ref) => {
		impl HtmlAttributeValue for $ty {
			fn fmt(self, formatter: &mut HtmlAttributeFormatter) -> fmt::Result {
				formatter.write(self.as_ref());
				Ok(())
			}
		}

		impl HtmlTemplate for $ty {
			fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
				formatter.write(self.as_ref());
				Ok(())
			}
		}
	};

	($ty:ty, raw Display) => {
		impl HtmlAttributeValue for $ty {
			fn fmt(self, formatter: &mut HtmlAttributeFormatter) -> fmt::Result {
				display(format_args!("{}", self), |value| {
					formatter.write_bytes(value)
				})
			}
		}

		impl HtmlTemplate for $ty {
			fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
				display(format_args!("{}", self), |value| {
					formatter.write_bytes(value)
				})
			}
		}
	};
}

impl_simple_write!(String, as_ref);
impl_simple_write!(&str, as_ref);
impl_simple_write!(bool, raw Display);
impl_simple_write!(u8, raw Display);
impl_simple_write!(u16, raw Display);
impl_simple_write!(u32, raw Display);
impl_simple_write!(u64, raw Display);
impl_simple_write!(u128, raw Display);
impl_simple_write!(usize, raw Display);
impl_simple_write!(i8, raw Display);
impl_simple_write!(i16, raw Display);
impl_simple_write!(i32, raw Display);
impl_simple_write!(i64, raw Display);
impl_simple_write!(i128, raw Display);
impl_simple_write!(isize, raw Display);
impl_simple_write!(f32, raw Display);
impl_simple_write!(f64, raw Display);

impl<F> HtmlTemplate for F
where
	F: FnOnce(&mut HtmlFormatter) -> fmt::Result,
{
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		self(formatter)
	}
}

pub struct HtmlIterTemplate<I>(I);

impl<I> HtmlTemplate for HtmlIterTemplate<I>
where
	I: IntoIterator,
	I::Item: HtmlTemplate,
{
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		for item in self.0 {
			item.fmt(formatter)?;
		}

		Ok(())
	}
}

pub trait IntoHtmlTemplate {
	type Template: HtmlTemplate;

	fn into_template(self) -> Self::Template;
}

impl<I> IntoHtmlTemplate for I
where
	I: IntoIterator,
	I::Item: HtmlTemplate,
{
	type Template = HtmlIterTemplate<I>;

	fn into_template(self) -> Self::Template {
		HtmlIterTemplate(self)
	}
}

// pub struct HtmlWriter<'a, W> {
// 	writer: &'a mut W,
// }

// impl<'a, W> HtmlWriter<'a, W> {
// 	pub fn new(writer: &'a mut W) -> Self {
// 		Self { writer }
// 	}
// }

// impl<'a, W: BufMut> HtmlFormatter for HtmlWriter<'a, W> {
// 	type Error = fmt::Error;

// 	fn write_raw(&mut self, raw: &str) -> Result<(), Self::Error> {
// 		self.writer.put_slice(raw.as_bytes());
// 		Ok(())
// 	}

// 	fn write_comment(&mut self, _comment: &str) -> Result<(), Self::Error> {
// 		// don't write comments in production
// 		Ok(())
// 	}
// }

// #[pin_project]
// struct HtmlTemplateBody<T: HtmlTemplate>(Option<T>);

// impl<T: HtmlTemplate> Future for HtmlTemplateBody<T> {
// 	type Output = Option<Result<Bytes, Infallible>>;

// 	fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
// 		Poll::Ready(self.project().0.take().map(|template| {
// 			let mut bytes = BytesMut::new();
// 			let mut writer = HtmlWriter::new(&mut bytes);
// 			template.fmt(&mut writer).unwrap();
// 			Ok(bytes.freeze())
// 		}))
// 	}
// }

// impl<T: HtmlTemplate> FusedFuture for HtmlTemplateBody<T> {
// 	fn is_terminated(&self) -> bool {
// 		self.0.is_none()
// 	}
// }

// impl<T: HtmlTemplate> Body for HtmlTemplateBody<T> {
// 	type Data = Bytes;
// 	type Error = Infallible;

// 	fn poll_data(
// 		self: Pin<&mut Self>,
// 		cx: &mut Context<'_>,
// 	) -> Poll<Option<Result<Self::Data, Self::Error>>> {
// 		self.poll(cx)
// 	}

// 	fn poll_trailers(
// 		self: Pin<&mut Self>,
// 		_cx: &mut Context<'_>,
// 	) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
// 		Poll::Ready(Ok(None))
// 	}
// }

// impl<T: HtmlTemplate + Send + 'static> IntoResponse for HtmlTemplateBody<T> {
// 	fn into_response(self) -> Response {
// 		Response::new(body::boxed(self))
// 	}
// }

// pub trait HtmlTemplateExt: HtmlTemplate + Send + 'static {
// 	fn into_response(self) -> Response;
// }

// impl<T: HtmlTemplate + Send + Sized + 'static> HtmlTemplateExt for T {
// 	fn into_response(self) -> Response {
// 		let mut response = Response::new(body::boxed(HtmlTemplateBody(Some(self))));
// 		response.headers_mut().insert(
// 			header::CONTENT_TYPE,
// 			HeaderValue::from_static("text/html; charset=utf-8"),
// 		);
// 		response
// 	}
// }
