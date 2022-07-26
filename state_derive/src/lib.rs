use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Derive macro generating an impl of the trait `state_lock::State`
#[proc_macro_derive(State, attributes(family))]
pub fn derive_state(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    // eprintln!("==========================================================");
    // eprintln!("{:#?}", ast);
    // eprintln!("==========================================================");
    let struct_ident = ast.ident;

    let impl_mod = syn::Ident::new(&format!("{}_impl", struct_ident), struct_ident.span());

    let family_attr = get_attr("family", ast.attrs);
    let family = match get_family_from_attr(family_attr) {
        Err(e) => return e.to_compile_error().into(),
        Ok(f) => f,
    };

    let out = quote!(
        pub use #impl_mod::*;
        #[allow(non_snake_case)]
        mod #impl_mod {
            use super::*;
            impl state_lock::State for super::#struct_ident {
                fn state_name() -> &'static str {
                    stringify!(#struct_ident)
                }
                fn name(&self) -> &'static str {
                    Self::state_name()
                }
                fn family(&self) -> &'static str {
                    #family
                }
                fn tear_up() -> Self {
                    Self::default()
                }
                fn as_any(&self) -> &dyn std::any::Any {
                    self
                }
            }
            impl super::#struct_ident {
                fn create_default() -> Box<dyn state_lock::State> {
                    Box::new(Self::default())
                }
            }
            state_lock::inventory::submit! {
                state_lock::StateRegistration {
                    state: stringify!(#struct_ident),
                    state_family: #family,
                    tear_up_fn: super::#struct_ident::create_default,
                }
            }
        }
    );
    // eprintln!("{}", out);
    out.into()
}

macro_rules! bail {
    ($t: expr, $msg: expr) => {
        return Err(attr_error($t, $msg))
    };
}

fn attr_error<T: quote::ToTokens>(tokens: T, message: &str) -> syn::Error {
    syn::Error::new_spanned(tokens, message)
}

fn get_attr(attr_ident: &str, attrs: Vec<syn::Attribute>) -> Option<syn::Attribute> {
    attrs
        .into_iter()
        .find(|attr| attr.path.segments.len() == 1 && attr.path.segments[0].ident == attr_ident)
}

fn get_family_from_attr(attr: Option<syn::Attribute>) -> Result<syn::NestedMeta, syn::Error> {
    if attr.is_none() {
        bail!(attr, "expected `family(state_family)`");
    }

    let meta = attr.unwrap().parse_meta();
    // eprintln!("{:#?}", meta);
    match meta {
        Ok(syn::Meta::List(meta_list)) => {
            // We expect only one expression
            if meta_list.nested.len() != 1 {
                bail!(meta_list.nested, "expected `family(state_family)`");
            }
            // Expecting `family()`
            Ok(meta_list.nested[0].clone())
        }
        _ => bail!("", "expected `family(state_family)`"),
    }
}