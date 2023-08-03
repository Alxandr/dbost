mod components;
mod templates;

#[proc_macro]
pub fn component(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
	components::define_component(tokens.into()).into()
}

#[test]
fn ui() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/compile_fail/*.rs");
}
