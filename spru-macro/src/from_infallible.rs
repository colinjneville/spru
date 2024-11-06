use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Item};

pub(crate) fn derive_from_infallible(input: TokenStream) -> syn::Result<TokenStream> {
    let item: Item = parse2(input)?;
    let (ident, generics) = match &item {
        Item::Enum(item) => (&item.ident, &item.generics),
        Item::Struct(item) => (&item.ident, &item.generics),
        Item::Union(item) => (&item.ident, &item.generics),
        _ => unreachable!("Derive macros can only be applied to enums, structs, and unions"),
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::std::convert::From<::std::convert::Infallible> for #ident #ty_generics
        #where_clause {
            fn from(value: ::std::convert::Infallible) -> Self {
                ::std::unreachable!()
            }
        }
    })
}