use html_template::{HtmlAttributeValue, HtmlTemplate};
use html_template_macro::component;

component! {
	struct NavBar {
		<nav>
			<ul>
				<li><a href="/">Home</a></li>
				<li><a href="/about">About</a></li>
			</ul>
		</nav>
	}
}

component! {
	/// This is some comments
	#[derive(Debug)]
	pub(crate) struct Template<T, A, C>(
		title: T,
		attribute: A,
		children: C
	)
	where
		T: Into<String>,
		A: HtmlAttributeValue + HtmlTemplate + Clone,
		C: HtmlTemplate,
	{
		<!DOCTYPE html>
		<html>
			<head>
				<title>{title.into()}</title>
			</head>
			<body>
				<!-- "comment" -->
				<div hello=attribute.clone() />
				<div hello=attribute.clone() />
				<>
					<div>"1"</div>
					<div> Hello  world with spaces </div>
					<div>"3"</div>
					<div>{attribute}</div>
					// <div {"some-attribute-from-rust-block"}/>
					<hr><hr />
				</>

				<main>
					<NavBar />
					{children}
				</main>
			</body>
		</html>
	}
}

component! {
	struct Page(title: impl Into<String>, heading: String) {
		<Template title=title attribute="world">
			<h1>{heading}</h1>
			<p>"This is a test"</p>
		</Template>
	}
}

#[test]
fn test_output() {
	let page = Page::new("Hello", "Hello world".into());
	let output = page
		.into_string()
		.expect("formatting works and produces valid utf-8");

	let expected = r#"
	<!DOCTYPE html>
	<html>
		<head>
			<title>Hello</title>
		</head>
		<body>
			<!--comment-->
			<div hello="world"></div>
			<div hello="world"></div>
			<div>1</div>
			<div>Hello world with spaces</div>
			<div>3</div>
			<div>world</div>
			<hr />
			<hr />
			<main>
				<nav>
					<ul>
						<li><a href="/">Home</a></li>
						<li><a href="/about">About</a></li>
					</ul>
				</nav>
				<h1>Hello world</h1>
				<p>This is a test</p>
			</main>
		</body>
	</html>
	"#
	.trim()
	.lines()
	.map(|l| l.trim())
	.collect::<String>();

	assert_eq!(output, expected);
}
