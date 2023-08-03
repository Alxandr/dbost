use rstml::node::Node;
use std::collections::HashSet;
use syn::{
	parse::Parse,
	punctuated::Punctuated,
	token::{Brace, Paren},
	Attribute, Generics, Ident, Token, Type, Visibility,
};

pub struct ComponentProp {
	pub name: Ident,
	pub colon: Token![:],
	pub type_bound: Type,
}

impl Parse for ComponentProp {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let name: Ident = input.parse()?;
		let colon = input.parse()?;
		let type_bound = input.parse()?;

		Ok(Self {
			name,
			colon,
			type_bound,
		})
	}
}

pub struct ComponentProps {
	pub paren_token: Paren,
	pub props: Punctuated<ComponentProp, Token![,]>,
}

impl Parse for ComponentProps {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let content;

		Ok(Self {
			paren_token: syn::parenthesized!(content in input),
			props: content.parse_terminated(ComponentProp::parse, Token![,])?,
		})
	}
}

// example input:
// component!{
//   pub(crate) struct TestComponent<T>(title: T, children: impl HtmlTemplate) where T: Into<String> {
//     <main>
//       <h1>{title.into()}</h1>
//       {children}
//     </main>
//   }
// }
pub struct Component {
	pub attributes: Vec<Attribute>,
	pub visibility: Visibility,
	pub struct_token: Token![struct],
	pub name: Ident,
	pub generics: Generics,
	pub props: Option<ComponentProps>,
	pub brace_token: Brace,
	pub body: rstml::ParsingResult<Vec<Node>>,
}

impl Parse for Component {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let attributes = Attribute::parse_outer(input)?;
		let visibility: Visibility = input.parse()?;
		let struct_token: Token![struct] = input.parse()?;
		let name: Ident = input.parse()?;
		let mut generics: Generics = input.parse()?;
		let props: Option<ComponentProps> = if input.peek(Paren) {
			Some(input.parse()?)
		} else {
			None
		};

		generics.where_clause = input.parse()?;

		let content;
		let brace_token = syn::braced!(content in input);

		let config = rstml::ParserConfig::new()
			.recover_block(true)
			.always_self_closed_elements(empty_elements())
			.raw_text_elements(["script", "style"].into_iter().collect());

		let parser = rstml::Parser::new(config);
		let template = parser.parse_syn_stream(&content);

		Ok(Self {
			attributes,
			visibility,
			struct_token,
			name,
			generics,
			props,
			brace_token,
			body: template,
		})
	}
}

pub fn empty_elements() -> HashSet<&'static str> {
	// https://developer.mozilla.org/en-US/docs/Glossary/Empty_element
	[
		"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
		"track", "wbr",
	]
	.into_iter()
	.collect()
}
