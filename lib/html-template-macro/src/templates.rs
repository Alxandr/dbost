use self::ide::IdeHelper;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2_diagnostics::Diagnostic;
use quote::quote;
use quote::ToTokens;
use rstml::node::{NodeBlock, NodeComment, NodeName, NodeText, RawText};
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::Block;
use syn::Expr;
use syn::LitByteStr;
use syn::Path;
use syn::Stmt;

mod ide;
mod parsing;

pub use parsing::TemplateParser;

pub trait NodeVisitor {
	fn visit_attribute(&mut self, expr: &Expr);
	fn visit_template(&mut self, expr: &Expr);
}

// struct TemplateCapture {
// 	ty: GenericParam,
// 	expr: Expr,
// }
struct Children {
	instructions: Vec<TemplateWriteInstruction>,
}

enum AttributeValue {
	Constant(String),
	Expression(Expr),
}

enum TemplateWriteInstruction {
	Doctype(RawText),
	OpenTagStart(NodeName),
	AttributeName(NodeName),
	AttributeValue(AttributeValue),
	OpenTagEnd,
	SelfCloseTag,
	EndTag(NodeName),
	Text(NodeText),
	RawText(RawText),
	Comment(NodeComment),
	Content(NodeBlock),
	Component {
		path: Path,
		props: Vec<(Ident, Expr)>,
		children: Option<Children>,
	},
}

pub struct Template {
	instructions: Vec<TemplateWriteInstruction>,
	diagnostics: Vec<Diagnostic>,
	ide_helper: IdeHelper,
}

impl Template {
	pub fn parser<'a>(empty_elements: &'a HashSet<&'a str>) -> TemplateParser<'a> {
		TemplateParser::new(empty_elements)
	}

	pub fn with_formatter<'a>(&'a self, formatter: &'a Ident) -> impl ToTokens + 'a {
		TemplateTokensWriter {
			instructions: &self.instructions,
			diagnostics: &self.diagnostics,
			ide_helper: &self.ide_helper,
			formatter,
		}
	}

	pub fn extend_diagnostics(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
		self.diagnostics.extend(diagnostics);
	}
}

struct TemplateTokensWriter<'a> {
	instructions: &'a [TemplateWriteInstruction],
	diagnostics: &'a [Diagnostic],
	ide_helper: &'a IdeHelper,
	formatter: &'a Ident,
}

struct TemplateInstructionWriter<'a> {
	instruction: &'a TemplateWriteInstruction,
	formatter: &'a Ident,
}

impl<'a> ToTokens for TemplateTokensWriter<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let instructions = self.instructions.iter().map(|i| TemplateInstructionWriter {
			instruction: i,
			formatter: self.formatter,
		});
		let diagnostics = self
			.diagnostics
			.iter()
			.map(|d| d.clone().emit_as_item_tokens());
		let ide_helper = self.ide_helper;

		tokens.extend(quote! {
			#ide_helper
			#(#instructions)*
			#(#diagnostics)*
		});
	}
}

impl<'a> ToTokens for TemplateInstructionWriter<'a> {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let formatter = self.formatter;
		match self.instruction {
			TemplateWriteInstruction::Doctype(doctype) => {
				let value = &doctype.to_token_stream_string();
				let value = LitByteStr::new(value.as_bytes(), doctype.span());
				tokens.extend(quote!(#formatter.write_doctype(#value);));
			}
			TemplateWriteInstruction::OpenTagStart(name) => {
				let value = name.to_string();
				let value = LitByteStr::new(value.as_bytes(), Span::call_site());
				tokens.extend(quote!(#formatter.write_open_tag_start(#value);));
			}
			TemplateWriteInstruction::AttributeName(name) => {
				let value = name.to_string();
				let value = LitByteStr::new(value.as_bytes(), Span::call_site());
				tokens.extend(quote!(#formatter.write_attribute_name(#value);));
			}
			TemplateWriteInstruction::AttributeValue(expr) => {
				tokens.extend(quote!(#formatter.write_attribute_value(#expr)?;));
			}
			TemplateWriteInstruction::OpenTagEnd => {
				tokens.extend(quote!(#formatter.write_open_tag_end();));
			}
			TemplateWriteInstruction::SelfCloseTag => {
				tokens.extend(quote!(#formatter.write_self_close_tag();));
			}
			TemplateWriteInstruction::EndTag(name) => {
				let value = name.to_string();
				let value = LitByteStr::new(value.as_bytes(), Span::call_site());
				tokens.extend(quote!(#formatter.write_end_tag(#value);));
			}
			TemplateWriteInstruction::Text(content) => {
				let value = content.value_string();
				let value = LitByteStr::new(value.as_bytes(), content.span());
				tokens.extend(quote!(#formatter.write_bytes(#value);));
			}
			TemplateWriteInstruction::RawText(content) => {
				let value = content.to_string_best();
				let value = LitByteStr::new(value.as_bytes(), content.span());
				tokens.extend(quote!(#formatter.write_bytes(#value);));
			}
			TemplateWriteInstruction::Comment(comment) => {
				let value = &comment.value;
				let value = LitByteStr::new(value.value().as_bytes(), comment.value.span());
				tokens.extend(quote!(#formatter.write_comment(#value);));
			}
			TemplateWriteInstruction::Content(content) => {
				match content {
					NodeBlock::ValidBlock(Block { stmts, .. }) if stmts.len() == 1 => {
						if let Stmt::Expr(expr, None) = &stmts[0] {
							tokens.extend(quote!(#formatter.write_content(#expr)?;));
							return;
						}
					}
					_ => (),
				}

				tokens.extend(quote!(#formatter.write_content(#content)?;));
			}
			TemplateWriteInstruction::Component {
				path: name,
				props,
				children,
			} => {
				let mut props = props
					.iter()
					.map(|(name, expr)| quote!(#name: #expr))
					.collect::<Vec<_>>();

				if let Some(Children { instructions }) = children {
					// let mut captures = Vec::new();
					let instructions = instructions
						.iter()
						.map(|i| TemplateInstructionWriter {
							instruction: i,
							formatter: self.formatter,
						})
						.collect::<Vec<_>>();

					let children = quote!(|#formatter: &mut ::html_template::HtmlFormatter| {
						#(#instructions)*
						Ok(())
					});

					props.push(quote!(children: #children));
				}

				tokens.extend(quote!(#formatter.write_content(#name { #(#props),* })?;));
			}
		}
	}
}

impl ToTokens for AttributeValue {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		match self {
			AttributeValue::Constant(value) => {
				let value = &**value;
				tokens.extend(quote!(#value));
			}
			AttributeValue::Expression(expr) => {
				tokens.extend(quote!(#expr));
			}
		}
	}
}
