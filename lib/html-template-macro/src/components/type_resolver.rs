use super::parsing;
use convert_case::{Case, Casing};
use indexmap::IndexMap;
use proc_macro2::Span;
use proc_macro2_diagnostics::{Diagnostic, SpanDiagnosticExt};
use syn::{
	spanned::Spanned, GenericParam, Generics, Ident, Path, PredicateType, Type, TypeImplTrait,
	TypeParam, TypePath, WhereClause, WherePredicate,
};

pub struct PropTypeResolver {
	unit_struct: bool,
	types: IndexMap<Ident, Type>,
	generics: Generics,
	diagnostics: Vec<Diagnostic>,
}

impl PropTypeResolver {
	pub fn new(component_props: Option<parsing::ComponentProps>, mut generics: Generics) -> Self {
		let mut diagnostics = Vec::new();
		let mut has_mixed_generics = false;
		let mixed_generics_diagnostic = generics
			.span()
			.error("Cannot use impl traits *and* explicit generics in one component.");

		match component_props {
			None => Self {
				unit_struct: true,
				types: IndexMap::new(),
				generics,
				diagnostics,
			},

			Some(value) => {
				let has_explicit_generics = !generics.params.is_empty();

				let types = value
					.props
					.into_iter()
					.map(|prop| {
						let name = prop.name;
						let tb = prop.type_bound;
						let prop_type = match tb {
							Type::ImplTrait(impl_trait) => {
								if has_explicit_generics {
									has_mixed_generics = true;
									diagnostics.push(
										impl_trait
											.span()
											.error("Cannot use impl traits *and* explicit generics at the same time"),
									);
								}

								let TypeImplTrait { bounds, .. } = impl_trait;
								let generic_param_ident = Ident::new(
									&format!("T{}", name.to_string().to_case(Case::Pascal)),
									Span::call_site(),
								);

								let prop_type = Type::Path(TypePath {
									qself: None,
									path: Path::from(generic_param_ident.clone()),
								});

								generics.params.push(GenericParam::Type(TypeParam {
									attrs: vec![],
									ident: generic_param_ident,
									colon_token: Default::default(),
									bounds: Default::default(),
									eq_token: None,
									default: None,
								}));

								generics
									.where_clause
									.get_or_insert_with(|| WhereClause {
										where_token: Default::default(),
										predicates: Default::default(),
									})
									.predicates
									.push(WherePredicate::Type(PredicateType {
										lifetimes: None,
										bounded_ty: prop_type.clone(),
										colon_token: Default::default(),
										bounds,
									}));

								prop_type
							}

							t => t,
						};

						(name, prop_type)
					})
					.collect();

				if has_mixed_generics {
					diagnostics.push(mixed_generics_diagnostic);
				}

				Self {
					unit_struct: false,
					types,
					generics,
					diagnostics,
				}
			}
		}
	}

	pub fn complete(self) -> ResolvedProps {
		if self.unit_struct {
			return ResolvedProps::new(self.generics, None, self.diagnostics);
		}

		let generics = self.generics;
		let types = self.types.into_iter().collect::<Vec<_>>();

		ResolvedProps::new(generics, Some(types), self.diagnostics)
	}
}

pub struct ResolvedProps {
	pub generics: Generics,
	pub props: Option<Vec<(Ident, Type)>>,
	pub diagnostics: Vec<Diagnostic>,
}

impl ResolvedProps {
	fn new(
		generics: Generics,
		props: Option<Vec<(Ident, Type)>>,
		diagnostics: Vec<Diagnostic>,
	) -> Self {
		Self {
			generics,
			props,
			diagnostics,
		}
	}
}
