use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Derive macro generating an impl of the trait `state_lock::State`
#[proc_macro_derive(State)]
pub fn derive_state(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    // eprintln!("==========================================================");
    // eprintln!("{:#?}", ast);
    // eprintln!("==========================================================");
    let struct_ident = ast.ident;
    let impl_mod = syn::Ident::new(&format!("{}_impl", struct_ident), struct_ident.span());

    let out = quote!(
        pub use #impl_mod::*;
        #[allow(non_snake_case)]
        mod #impl_mod {
            impl state_lock::State for super::#struct_ident {
                fn state_name() -> &'static str {
                    stringify!(#struct_ident)
                }
                fn name(&self) -> &'static str {
                    Self::state_name()
                }
                fn tear_up() -> Self {
                    Self::default()
                }
                fn as_any(&self) -> &dyn std::any::Any {
                    self
                }
            }

            impl super::#struct_ident {
                fn crate_default() -> Box<dyn state_lock::State> {
                    Box::new(Self::default())
                }
            }
            state_lock::inventory::submit! {
                state_lock::StateRegistration {
                    state: stringify!(#struct_ident),
                    tear_up_fn: super::#struct_ident::crate_default,
                }
            }
        }
    );
    // eprintln!("{}", out);
    out.into()
}
