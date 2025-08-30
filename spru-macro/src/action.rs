use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, parse_quote, spanned::Spanned as _, Item};

pub(crate) fn derive_create(item: TokenStream) -> syn::Result<TokenStream> {
    derive_action(item, parse_quote!(::spru::action::Create))
}

pub(crate) fn derive_update(item: TokenStream) -> syn::Result<TokenStream> {
    derive_action(item, parse_quote!(::spru::action::Update))
}

pub(crate) fn derive_destroy(item: TokenStream) -> syn::Result<TokenStream> {
    derive_action(item, parse_quote!(::spru::action::Destroy))
}

fn derive_action(item: TokenStream, trait_path: syn::Path) -> syn::Result<TokenStream> {
    let item: Item = parse2(item)?;
    let (item_ident, generics) = match &item {
        Item::Enum(item_enum) => Ok((&item_enum.ident, &item_enum.generics)),
        Item::Struct(item_struct) => Ok((&item_struct.ident, &item_struct.generics)),
        Item::Union(item_union) => Ok((&item_union.ident, &item_union.generics)),
        _ => Err(syn::Error::new(item.span(), "Expected an enum, struct, or union")),
    }?;

    let lookup_ident = quote!(Lookup);

    let mut generics = generics.clone();
    generics.make_where_clause().predicates.push(parse_quote!(Self: #trait_path));

    let mut lookup_generics = generics.clone();
    lookup_generics.params.push(parse_quote!(#lookup_ident));
    lookup_generics.make_where_clause().predicates.push(parse_quote!(#lookup_ident: ::spru::item::lookup::OfType<<Self as #trait_path>::Data>));

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let (lookup_impl_generics, _lookup_type_generics, lookup_where_clause) = lookup_generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics ::spru::catalog::entry::Base for #item_ident #type_generics 
        #where_clause {
            type Out = <Self as #trait_path>::Undo;
            type Error = <Self as #trait_path>::Error;
        }

        impl #lookup_impl_generics ::spru::catalog::Entry<#lookup_ident> for #item_ident #type_generics 
        #lookup_where_clause {
            fn entry(&self, context: ::spru::catalog::Context<#lookup_ident>) -> ::std::result::Result<::std::option::Option<Self::Out>, ::spru::action::Error<#lookup_ident::Error, Self::Error>> {
                <Self as #trait_path>::_entry(self, context)
            }
        }
    })
}