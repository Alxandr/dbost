use std::convert::Infallible;

use html_template::{HtmlFormatter, HtmlTemplate};
use html_template_macro::template;

#[derive(Debug, PartialEq)]
enum FormatCalls {
	Raw(String),
	Doctype(String),
	OpenTagStart(String),
	AttributeName(String),
	AttributeValue(String),
	OpenTagEnd(bool),
	EndTag(String),
	Text(String),
	Comment(String),
}

#[derive(Default)]
struct FakeHtmlFormatter(Vec<FormatCalls>);

impl HtmlFormatter for FakeHtmlFormatter {
	type Error = Infallible;

	fn write_raw(&mut self, value: &str) -> Result<(), Infallible> {
		self.0.push(FormatCalls::Raw(value.into()));
		Ok(())
	}

	fn write_doctype(&mut self, value: &str) -> Result<(), Infallible> {
		self.0.push(FormatCalls::Doctype(value.into()));
		Ok(())
	}

	fn write_open_tag_start(&mut self, tag: &str) -> Result<(), Infallible> {
		self.0.push(FormatCalls::OpenTagStart(tag.into()));
		Ok(())
	}

	fn write_attribute_name(&mut self, name: &str) -> Result<(), Infallible> {
		self.0.push(FormatCalls::AttributeName(name.into()));
		Ok(())
	}

	fn write_attribute_value(&mut self, value: &str) -> Result<(), Infallible> {
		self.0.push(FormatCalls::AttributeValue(value.into()));
		Ok(())
	}

	fn write_open_tag_end(&mut self, self_closing: bool) -> Result<(), Infallible> {
		self.0.push(FormatCalls::OpenTagEnd(self_closing));
		Ok(())
	}

	fn write_end_tag(&mut self, tag: &str) -> Result<(), Infallible> {
		self.0.push(FormatCalls::EndTag(tag.into()));
		Ok(())
	}

	fn write_text(&mut self, text: &str) -> Result<(), Infallible> {
		self.0.push(FormatCalls::Text(text.into()));
		Ok(())
	}

	fn write_comment(&mut self, comment: &str) -> Result<(), Infallible> {
		self.0.push(FormatCalls::Comment(comment.into()));
		Ok(())
	}
}

#[test]
fn simple_template() {
	let world1 = "world1";
	let world2 = "world2";
	let world3 = "world3";
	let world4 = "world4";
	let template = template!(
		<!DOCTYPE html>
		<html>
			<head>
				<title>"Example"</title>
			</head>
			<body>
				<!-- "comment" -->
				<div hello=world1 />
				<div hello={world2} />
				<>
					<div>"1"</div>
					<div> Hello  world with spaces </div>
					<div>"3"</div>
					<div>{world3}</div>
					// <div {"some-attribute-from-rust-block"}/>
					<script>{world4}</script>
					<hr>
				</>
			</body>
		</html>
	);

	let mut formatter = FakeHtmlFormatter::default();
	template.fmt(&mut formatter).unwrap();

	let expected = vec![
		FormatCalls::Doctype("html".into()),
		FormatCalls::OpenTagStart("html".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::OpenTagStart("head".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::OpenTagStart("title".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::Text("Example".into()),
		FormatCalls::EndTag("title".into()),
		FormatCalls::EndTag("head".into()),
		FormatCalls::OpenTagStart("body".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::Comment("comment".into()),
		FormatCalls::OpenTagStart("div".into()),
		FormatCalls::AttributeName("hello".into()),
		FormatCalls::AttributeValue("world1".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::EndTag("div".into()),
		FormatCalls::OpenTagStart("div".into()),
		FormatCalls::AttributeName("hello".into()),
		FormatCalls::AttributeValue("world2".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::EndTag("div".into()),
		FormatCalls::OpenTagStart("div".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::Text("1".into()),
		FormatCalls::EndTag("div".into()),
		FormatCalls::OpenTagStart("div".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::Raw("Hello world with spaces".into()),
		FormatCalls::EndTag("div".into()),
		FormatCalls::OpenTagStart("div".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::Text("3".into()),
		FormatCalls::EndTag("div".into()),
		FormatCalls::OpenTagStart("div".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::Text("world3".into()),
		FormatCalls::EndTag("div".into()),
		FormatCalls::OpenTagStart("script".into()),
		FormatCalls::OpenTagEnd(false),
		FormatCalls::Raw("{world4}".into()),
		FormatCalls::EndTag("script".into()),
		FormatCalls::OpenTagStart("hr".into()),
		FormatCalls::OpenTagEnd(true),
		FormatCalls::EndTag("body".into()),
		FormatCalls::EndTag("html".into()),
	];

	for (idx, (actual, expected)) in formatter.0.iter().zip(expected.iter()).enumerate() {
		assert_eq!(actual, expected, "at index {}", idx);
	}

	assert_eq!(formatter.0.len(), expected.len());
}
