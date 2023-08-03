use crate::components::type_resolver::PropTypeResolver;
use crate::templates::Template;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::Attribute;
use syn::Generics;
use syn::{Ident, Type, Visibility};

mod parsing;
mod type_resolver;

pub struct ComponentProp {
	pub name: Ident,
	pub prop_type: Type,
}

impl ToTokens for ComponentProp {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let name = &self.name;
		let prop_type = &self.prop_type;
		tokens.extend(quote!(#name: #prop_type));
	}
}

pub struct Component {
	pub attributes: Vec<Attribute>,
	pub visibility: Visibility,
	pub name: Ident,
	pub generics: Generics,
	pub props: Option<Vec<ComponentProp>>,
	pub template: Template,
}

impl TryFrom<TokenStream> for Component {
	type Error = syn::Error;

	fn try_from(value: TokenStream) -> Result<Self, Self::Error> {
		let input: parsing::Component = syn::parse2(value)?;

		let attributes = input.attributes;
		let visibility = input.visibility;
		let name = input.name;

		let resolver = PropTypeResolver::new(input.props, input.generics);
		let mut template = Template::parser(&parsing::empty_elements()).parse(input.body);
		let props = resolver.complete();
		template.extend_diagnostics(props.diagnostics);

		let generics = props.generics;
		let props = props.props.map(|props| {
			props
				.into_iter()
				.map(|(name, prop_type)| {
					// TODO: If name is a reserved identifier, add diagnostics to the template
					ComponentProp { name, prop_type }
				})
				.collect()
		});

		Ok(Self {
			attributes,
			visibility,
			name,
			generics,
			props,
			template,
		})
	}
}

impl ToTokens for Component {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let formatter = Ident::new("__html", Span::call_site());
		let visibility = &self.visibility;
		let name = &self.name;
		let template = &self.template.with_formatter(&formatter);
		let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

		match self.props.as_deref() {
			None => {
				tokens.extend(quote! {
					#visibility struct #name #ty_generics #where_clause;

					impl #impl_generics #name #ty_generics #where_clause {
						pub fn new() -> Self {
							Self
						}
					}

					impl #impl_generics ::html_template::HtmlTemplate for #name #ty_generics #where_clause {
						fn fmt(self, #formatter: &mut ::html_template::HtmlFormatter) -> ::std::fmt::Result {
							#template
							Ok(())
						}
					}
				});
			}

			Some(props) => {
				let prop_names = props.iter().map(|prop| &prop.name).collect::<Vec<_>>();
				let attributes = &*self.attributes;

				tokens.extend(quote! {
					#(#attributes)*
					#visibility struct #name #ty_generics #where_clause {
						pub #(#props),*
					}

					impl #impl_generics #name #ty_generics #where_clause {
						pub fn new(#(#props),*) -> Self {
							Self {
								#(#prop_names),*
							}
						}
					}

					impl #impl_generics ::html_template::HtmlTemplate for #name #ty_generics #where_clause {
						fn fmt(self, #formatter: &mut ::html_template::HtmlFormatter) -> ::std::fmt::Result {
							let Self { #(#prop_names),* } = self;
							#template
							Ok(())
						}
					}
				});
			}
		}

		tokens.extend(quote! {
			impl #impl_generics ::html_template::HtmlComponent for #name #ty_generics #where_clause {
				type Template = Self;

				fn into_template(self) -> Self::Template {
					self
				}
			}
		});
	}
}

pub fn define_component(tokens: TokenStream) -> TokenStream {
	let component = match Component::try_from(tokens) {
		Ok(component) => component,
		Err(e) => return e.to_compile_error(),
	};

	component.into_token_stream()
}
