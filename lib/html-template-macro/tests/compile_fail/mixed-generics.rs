use html_template::{HtmlAttributeValue, HtmlTemplate};
use html_template_macro::component;

component! {
	/// This is some comments
	#[derive(Debug)]
	pub(crate) struct TestComponent<T>(
		title: T,
		attribute: impl HtmlAttributeValue + HtmlTemplate,
		children: impl HtmlTemplate
	)
	where
		T: Into<String>,
	{
		<main />
	}
}

fn main() -> fmt::Result {
	unreachable!()
}
