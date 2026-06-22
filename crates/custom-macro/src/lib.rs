//! Core-owned proc-macros. `ErrorCode` reads a per-variant `#[error_code("…")]`
//! attribute and generates a `code(&self) -> &'static str` match. A variant
//! missing the attribute is a compile error, so the registry stays complete.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, LitStr};

/// Derive `code(&self) -> &'static str` from a per-variant `#[error_code("…")]`.
#[proc_macro_derive(ErrorCode, attributes(error_code))]
pub fn derive_error_code(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Data::Enum(data) = &input.data else {
        return syn::Error::new(input.span(), "ErrorCode can only be derived for enums")
            .to_compile_error()
            .into();
    };

    let mut arms = Vec::with_capacity(data.variants.len());
    for variant in &data.variants {
        let code = match variant_code(variant) {
            Ok(code) => code,
            Err(err) => return err.to_compile_error().into(),
        };
        let ident = &variant.ident;
        let bind = match &variant.fields {
            Fields::Named(_) => quote!({ .. }),
            Fields::Unnamed(_) => quote!((..)),
            Fields::Unit => quote!(),
        };
        arms.push(quote! { Self::#ident #bind => #code });
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Stable, low-cardinality telemetry code for this variant.
            pub fn code(&self) -> &'static str {
                match self {
                    #(#arms),*
                }
            }
        }
    }
    .into()
}

fn variant_code(variant: &syn::Variant) -> syn::Result<LitStr> {
    let attr = variant
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("error_code"))
        .ok_or_else(|| {
            syn::Error::new(
                variant.span(),
                format!(
                    "variant `{}` is missing #[error_code(\"…\")]",
                    variant.ident
                ),
            )
        })?;
    attr.parse_args::<LitStr>()
}
