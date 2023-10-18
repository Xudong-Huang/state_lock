use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Error, ParseStream, Result};
use syn::{parse_macro_input, parse_quote, Attribute, Path, Token};

/// Derive macro generating an impl of the trait `state_lock::State`
#[proc_macro_derive(State, attributes(family, state_lock))]
pub fn derive_state(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as syn::DeriveInput);
    // eprintln!("==========================================================");
    // eprintln!("{:#?}", ast);
    // eprintln!("==========================================================");
    let struct_ident = ast.ident;

    let impl_mod = syn::Ident::new(
        &format!("{struct_ident}_state_lock_impl"),
        struct_ident.span(),
    );

    // let state_lock_attr = get_attr("state_lock", ast.attrs);
    let state_lock_path = match state_lock_path(&mut ast.attrs) {
        Err(e) => return e.to_compile_error().into(),
        Ok(p) => p,
    };

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
            impl #state_lock_path::State for super::#struct_ident {
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
                    use #state_lock_path::default::{HasDefault, NoDefaultImplement};
                    HasDefault::<Self>::tear_up()
                }
            }
            impl super::#struct_ident {
                fn create_default() -> Box<dyn #state_lock_path::State> {
                    Box::new(Self::tear_up())
                }
            }

            #[#state_lock_path::linkme::distributed_slice(#state_lock_path::STATE_REGISTRATION)]
            #[linkme(crate = #state_lock_path::linkme)]
            static STATE: #state_lock_path::StateRegistration = #state_lock_path::StateRegistration {
                state: stringify!(#struct_ident),
                state_family: #family,
                tear_up_fn: super::#struct_ident::create_default,
            };
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
        .find(|attr| attr.path().segments.len() == 1 && attr.path().segments[0].ident == attr_ident)
}

fn get_family_from_attr(attr: Option<syn::Attribute>) -> Result<proc_macro2::TokenStream> {
    if attr.is_none() {
        bail!(attr, "expected `family(state_family)`");
    }

    let meta = attr.unwrap().meta;
    // eprintln!("{:#?}", meta);
    match meta {
        syn::Meta::List(meta_list) => {
            // We expect only one expression
            if meta_list.path.segments.len() != 1 || meta_list.path.segments[0].ident != "family" {
                bail!(meta_list, "expected `family(state_family)`");
            }

            // Expecting `family()`
            Ok(meta_list.tokens)
        }
        _ => bail!("", "expected `family(state_family)`"),
    }
}

// #[state_lock(crate = path::to::state_lock)]
fn state_lock_path(attrs: &mut Vec<Attribute>) -> Result<Path> {
    let mut state_lock_path = None;
    let mut errors: Option<Error> = None;

    attrs.retain(|attr| {
        if !attr.path().is_ident("state_lock") {
            return true;
        }
        match attr.parse_args_with(|input: ParseStream| {
            input.parse::<Token![crate]>()?;
            input.parse::<Token![=]>()?;
            input.call(Path::parse_mod_style)
        }) {
            Ok(path) => state_lock_path = Some(path),
            Err(err) => match &mut errors {
                None => errors = Some(err),
                Some(errors) => errors.combine(err),
            },
        }
        false
    });

    match errors {
        None => Ok(state_lock_path.unwrap_or_else(|| parse_quote!(::state_lock))),
        Some(errors) => Err(errors),
    }
}
