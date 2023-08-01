use proc_macro::TokenStream;
use proc_macro2_diagnostics::{Diagnostic, SpanDiagnosticExt};
use quote::{quote, quote_spanned, ToTokens};
use rstml::{
	node::{
		AttributeValueExpr, FnBinding, KeyedAttribute, KeyedAttributeValue, Node, NodeAttribute,
		NodeBlock, NodeComment, NodeDoctype, NodeElement, NodeFragment, NodeName, NodeText, RawText,
	},
	Parser, ParserConfig,
};
use std::collections::HashSet;
use syn::spanned::Spanned;

enum CapturedValue<'a> {
	Block(&'a NodeBlock),
	Expr(&'a syn::Expr),
}

impl<'a> From<&'a NodeBlock> for CapturedValue<'a> {
	fn from(block: &'a NodeBlock) -> Self {
		Self::Block(block)
	}
}

impl<'a> From<&'a syn::Expr> for CapturedValue<'a> {
	fn from(expr: &'a syn::Expr) -> Self {
		Self::Expr(expr)
	}
}

impl<'a> ToTokens for CapturedValue<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		match self {
			CapturedValue::Block(block) => {
				tokens.extend(quote!(#[allow(unused_braces)] #block));
			}
			CapturedValue::Expr(expr) => expr.to_tokens(tokens),
		}
	}
}

enum CaptureKind {
	Attribute,
	Template,
}

struct Capture<'a> {
	kind: CaptureKind,
	value_param: proc_macro2::Ident,
	generic_param: proc_macro2::Ident,
	captured_value: CapturedValue<'a>,
}

struct TemplateCaptures<'a> {
	captures: Vec<Capture<'a>>,
}

impl<'a> TemplateCaptures<'a> {
	fn new() -> Self {
		Self {
			captures: Vec::new(),
		}
	}

	#[must_use]
	fn push(
		&mut self,
		expr: impl Into<CapturedValue<'a>>,
		kind: CaptureKind,
	) -> proc_macro2::TokenStream {
		let index = self.captures.len();
		let captured_value: CapturedValue = expr.into();
		let value_param = proc_macro2::Ident::new(&format!("v{index}"), captured_value.span());
		let generic_param = proc_macro2::Ident::new(&format!("T{index}"), captured_value.span());
		let index = syn::Index::from(index);
		let expression = quote!(self.#index);

		self.captures.push(Capture {
			kind,
			generic_param,
			value_param,
			captured_value,
		});

		expression
	}

	fn generic_args(&self) -> GenericArgs<'_> {
		GenericArgs(self)
	}

	fn generic_params(&self) -> GenericParams<'_> {
		GenericParams(self)
	}

	fn new_args(&self) -> NewArgs<'_> {
		NewArgs(self)
	}

	fn new_arg_values(&self) -> NewArgValues<'_> {
		NewArgValues(self)
	}

	fn captured_values(&self) -> CapturedValues<'_> {
		CapturedValues(self)
	}
}

struct GenericArgs<'a>(&'a TemplateCaptures<'a>);
impl<'a> ToTokens for GenericArgs<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let idents = self.0.captures.iter().map(|capture| &capture.generic_param);
		tokens.extend(quote!(#(#idents,)*))
	}
}

struct GenericParams<'a>(&'a TemplateCaptures<'a>);
impl<'a> ToTokens for GenericParams<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let params = self.0.captures.iter().map(|capture| {
			let kind = match capture.kind {
				CaptureKind::Attribute => quote!(::html_template::HtmlAttributeValue),
				CaptureKind::Template => quote!(::html_template::HtmlTemplate),
			};
			let ident = &capture.generic_param;
			quote!(#ident: #kind)
		});

		tokens.extend(quote!(#(#params,)*))
	}
}

struct NewArgs<'a>(&'a TemplateCaptures<'a>);
impl<'a> ToTokens for NewArgs<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let args = self.0.captures.iter().map(|capture| {
			let ident = &capture.value_param;
			let ty = &capture.generic_param;
			quote!(#ident: #ty)
		});

		tokens.extend(quote!(#(#args,)*))
	}
}

struct NewArgValues<'a>(&'a TemplateCaptures<'a>);
impl<'a> ToTokens for NewArgValues<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let idents = self.0.captures.iter().map(|capture| &capture.value_param);
		tokens.extend(quote!(#(#idents,)*))
	}
}

struct CapturedValues<'a>(&'a TemplateCaptures<'a>);
impl<'a> ToTokens for CapturedValues<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let captured_values = self
			.0
			.captures
			.iter()
			.map(|capture| &capture.captured_value);
		tokens.extend(quote!(#(#captured_values,)*))
	}
}

enum AttributeValue {
	Constant(String),
	Expression(proc_macro2::TokenStream),
}

impl ToTokens for AttributeValue {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		match self {
			AttributeValue::Constant(value) => {
				tokens.extend(quote!(formatter.write_attribute_value(#value)?;));
			}
			AttributeValue::Expression(expr) => {
				tokens.extend(quote!({
					let value = #expr;
					::html_template::HtmlAttributeValue::fmt(value, formatter)?;
				}));
			}
		}
	}
}

#[allow(clippy::enum_variant_names)]
enum TemplateInstruction<'a> {
	WriteDoctype(&'a RawText),
	WriteOpenTagStart(&'a NodeName),
	WriteAttributeName(&'a NodeName),
	WriteAttributeValue(AttributeValue),
	WriteOpenTagEnd(bool /* self-close */),
	WriteEndTag(&'a NodeName),
	WriteText(&'a NodeText),
	WriteRawText(&'a RawText),
	WriteComment(&'a NodeComment),
	WriteTemplate(proc_macro2::TokenStream),
}

impl<'a> ToTokens for TemplateInstruction<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		match self {
			TemplateInstruction::WriteDoctype(doctype) => {
				let value = &doctype.to_token_stream_string();
				tokens.extend(quote!(formatter.write_doctype(#value)?;));
			}
			TemplateInstruction::WriteOpenTagStart(name) => {
				let name = name.to_string();
				tokens.extend(quote!(formatter.write_open_tag_start(#name)?;));
			}
			TemplateInstruction::WriteAttributeName(name) => {
				let name = name.to_string();
				tokens.extend(quote!(formatter.write_attribute_name(#name)?;));
			}
			TemplateInstruction::WriteAttributeValue(value) => {
				value.to_tokens(tokens);
			}
			TemplateInstruction::WriteOpenTagEnd(self_close) => {
				tokens.extend(quote!(formatter.write_open_tag_end(#self_close)?;));
			}
			TemplateInstruction::WriteEndTag(name) => {
				let name = name.to_string();
				tokens.extend(quote!(formatter.write_end_tag(#name)?;));
			}
			TemplateInstruction::WriteText(text) => {
				let value = text.value_string();
				tokens.extend(quote!(formatter.write_text(#value)?;));
			}
			TemplateInstruction::WriteRawText(raw_text) => {
				let value = raw_text.to_string_best();
				tokens.extend(quote!(formatter.write_raw(#value)?;));
			}
			TemplateInstruction::WriteComment(comment) => {
				let value = &comment.value;
				tokens.extend(quote!(formatter.write_comment(#value)?;));
			}
			TemplateInstruction::WriteTemplate(expr) => {
				tokens.extend(quote!({
					let value = #expr;
					::html_template::HtmlTemplate::fmt(value, formatter)?;
				}));
			}
		}
	}
}

#[derive(Default)]
struct IdeHelper<'a> {
	open_tag_names: Vec<&'a NodeName>,
	close_tag_names: Vec<&'a NodeName>,
	attr_names: Vec<&'a NodeName>,
}

impl<'a> IdeHelper<'a> {
	fn new() -> Self {
		Default::default()
	}
}

impl<'a> ToTokens for IdeHelper<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		fn mark_as_type(name: &NodeName) -> proc_macro2::TokenStream {
			let element = quote_spanned!(name.span() => enum);
			quote!({#element X{}})
		}

		fn mark_as_function(name: &NodeName) -> proc_macro2::TokenStream {
			let element = quote_spanned!(name.span() => checked_add);
			quote!({let _ = i32::#element;})
		}

		self
			.open_tag_names
			.iter()
			.for_each(|name| tokens.extend(mark_as_type(name)));
		self
			.close_tag_names
			.iter()
			.for_each(|name| tokens.extend(mark_as_type(name)));
		self
			.attr_names
			.iter()
			.for_each(|name| tokens.extend(mark_as_function(name)));
	}
}

enum NameKind {
	OpenTag,
	CloseTag,
	Attribute,
}

struct Template<'a> {
	captures: TemplateCaptures<'a>,
	instructions: Vec<TemplateInstruction<'a>>,

	// Additional diagnostic messages.
	diagnostics: Vec<Diagnostic>,

	empty_elements: &'a HashSet<&'a str>,

	ide_helper: IdeHelper<'a>,
}

impl<'a> Template<'a> {
	fn new(
		empty_elements: &'a HashSet<&'a str>,
		nodes: &'a [Node],
		diagnostics: Vec<Diagnostic>,
	) -> Self {
		let mut output = Self {
			captures: TemplateCaptures::new(),
			instructions: Vec::new(),
			diagnostics,
			empty_elements,
			ide_helper: IdeHelper::new(),
		};

		output.visit_nodes(nodes);
		output
	}

	fn visit_name(&mut self, name: &'a NodeName, kind: NameKind) -> &'a NodeName {
		fn is_valid(path: &syn::ExprPath) -> bool {
			if path.qself.is_some() {
				return false;
			}

			if !path.attrs.is_empty() {
				return false;
			}

			if path.path.leading_colon.is_some() {
				return false;
			}

			if path.path.segments.len() != 1 {
				return false;
			}

			true
		}

		match name {
			NodeName::Block(_) => {
				self
					.diagnostics
					.push(name.span().error("Only static names are supported."));
			}
			NodeName::Path(path) if !is_valid(path) => {
				self
					.diagnostics
					.push(name.span().error("Only static names are supported."));
			}
			_ => (),
		}

		match kind {
			NameKind::OpenTag => self.ide_helper.open_tag_names.push(name),
			NameKind::CloseTag => self.ide_helper.close_tag_names.push(name),
			NameKind::Attribute => self.ide_helper.attr_names.push(name),
		}

		name
	}

	fn visit_nodes(&mut self, nodes: &'a [Node]) {
		for node in nodes {
			self.visit_node(node);
		}
	}

	fn visit_node(&mut self, node: &'a Node) {
		match node {
			Node::Doctype(doctype) => self.visit_doctype(doctype),
			Node::Element(element) => self.visit_element(element),
			Node::Text(text) => self.visit_text(text),
			Node::RawText(raw_text) => self.visit_raw_text(raw_text),
			Node::Fragment(fragment) => self.visit_fragment(fragment),
			Node::Comment(comment) => self.visit_comment(comment),
			Node::Block(block) => self.visit_block(block),
		}
	}

	fn visit_doctype(&mut self, doctype: &'a NodeDoctype) {
		self
			.instructions
			.push(TemplateInstruction::WriteDoctype(&doctype.value));
	}

	fn visit_element(&mut self, element: &'a NodeElement) {
		let name = self.visit_name(element.name(), NameKind::OpenTag);
		if let Some(close_tag) = element.close_tag.as_ref() {
			self.visit_name(&close_tag.name, NameKind::CloseTag);
		}

		// TODO: disallow block names

		self
			.instructions
			.push(TemplateInstruction::WriteOpenTagStart(name));

		// attributes
		self.visit_attributes(name, element.attributes());

		if self.empty_elements.contains(&*name.to_string()) {
			// special empty tags that can't have children (for instance <br>)
			self
				.instructions
				.push(TemplateInstruction::WriteOpenTagEnd(true));

			if !element.children.is_empty() {
				self
					.diagnostics
					.push(element.span().error("Empty elements cannot have children."));
			}
		} else {
			// normal tags
			self
				.instructions
				.push(TemplateInstruction::WriteOpenTagEnd(false));

			// children
			self.visit_nodes(&element.children);

			// end tag
			self
				.instructions
				.push(TemplateInstruction::WriteEndTag(name));
		}
	}

	fn visit_attributes(&mut self, element_name: &'a NodeName, attributes: &'a [NodeAttribute]) {
		for attribute in attributes {
			// TODO: Special handling of class? and duplicates?
			self.visit_attribute(element_name, attribute);
		}
	}

	fn visit_attribute(&mut self, element_name: &'a NodeName, attribute: &'a NodeAttribute) {
		match attribute {
			NodeAttribute::Block(block) => self.visit_block_attribute(element_name, block),
			NodeAttribute::Attribute(attribute) => self.visit_static_attribute(element_name, attribute),
		}
	}

	fn visit_block_attribute(&mut self, _element_name: &'a NodeName, block: &'a NodeBlock) {
		// We do not support arbitrary attribute names.
		self.diagnostics.push(
			block
				.span()
				.error("Arbitrary attribute names are not supported."),
		);
	}

	fn visit_static_attribute(&mut self, element_name: &'a NodeName, attribute: &'a KeyedAttribute) {
		let attribute_name = self.visit_name(&attribute.key, NameKind::Attribute);
		self
			.instructions
			.push(TemplateInstruction::WriteAttributeName(attribute_name));

		match &attribute.possible_value {
			KeyedAttributeValue::Binding(binding) => {
				self.visit_attribute_binding(element_name, attribute_name, binding)
			}
			KeyedAttributeValue::Value(value) => {
				self.visit_attribute_value(element_name, attribute_name, value)
			}
			KeyedAttributeValue::None => (),
		}
	}

	fn visit_attribute_binding(
		&mut self,
		_element_name: &'a NodeName,
		_attribute_name: &'a NodeName,
		binding: &'a FnBinding,
	) {
		self.diagnostics.push(
			binding
				.span()
				.error("Attribute bindings are not supported."),
		);
	}

	fn visit_attribute_value(
		&mut self,
		_element_name: &'a NodeName,
		_attribute_name: &'a NodeName,
		value: &'a AttributeValueExpr,
	) {
		if let Some(value) = value.value_literal_string() {
			self
				.instructions
				.push(TemplateInstruction::WriteAttributeValue(
					AttributeValue::Constant(value),
				));
		} else {
			let capture = self.captures.push(&value.value, CaptureKind::Attribute);
			self
				.instructions
				.push(TemplateInstruction::WriteAttributeValue(
					AttributeValue::Expression(capture),
				));
		}
	}

	fn visit_text(&mut self, text: &'a NodeText) {
		self.instructions.push(TemplateInstruction::WriteText(text));
	}

	fn visit_raw_text(&mut self, raw_text: &'a RawText) {
		self
			.instructions
			.push(TemplateInstruction::WriteRawText(raw_text));
	}

	fn visit_fragment(&mut self, fragment: &'a NodeFragment) {
		self.visit_nodes(&fragment.children);
	}

	fn visit_comment(&mut self, comment: &'a NodeComment) {
		self
			.instructions
			.push(TemplateInstruction::WriteComment(comment));
	}

	fn visit_block(&mut self, block: &'a NodeBlock) {
		let capture = self.captures.push(block, CaptureKind::Template);
		self
			.instructions
			.push(TemplateInstruction::WriteTemplate(capture));
	}
}

impl<'a> ToTokens for Template<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let Template {
			captures,
			instructions,
			diagnostics,
			ide_helper,
			..
		} = self;

		let generic_args = captures.generic_args();
		let generic_params = captures.generic_params();
		let new_args = captures.new_args();
		let new_arg_values = captures.new_arg_values();
		let captured_values = captures.captured_values();
		let diagnostics = diagnostics.iter().cloned().map(|d| d.emit_as_item_tokens());

		tokens.extend(quote! {{
			#(#diagnostics)*
			#ide_helper

			struct Template<#generic_params>(#generic_args);
			impl<#generic_params> Template<#generic_args> {
				fn new(#new_args) -> impl ::html_template::HtmlTemplate {
					Self(#new_arg_values)
				}
			}

			impl<#generic_params> ::html_template::HtmlTemplate for Template<#generic_args> {
				fn fmt<F>(self, formatter: &mut F) -> Result<(), <F as ::html_template::HtmlFormatter>::Error> where F: ::html_template::HtmlFormatter {
					#(#instructions)*
					Ok(())
				}
			}

			Template::new(#captured_values)
		}});
	}
}

#[proc_macro]
pub fn template(tokens: TokenStream) -> TokenStream {
	// https://developer.mozilla.org/en-US/docs/Glossary/Empty_element
	let empty_elements: HashSet<_> = [
		"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
		"track", "wbr",
	]
	.into_iter()
	.collect();

	let config = ParserConfig::new()
		.recover_block(true)
		.always_self_closed_elements(empty_elements.clone())
		.raw_text_elements(["script", "style"].into_iter().collect());

	let parser = Parser::new(config);
	let (nodes, errors) = parser.parse_recoverable(tokens).split_vec();

	let output = Template::new(&empty_elements, &nodes, errors);
	output.into_token_stream().into()
}
