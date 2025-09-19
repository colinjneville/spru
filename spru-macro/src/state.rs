use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned as _};

pub(crate) fn derive_state(input: TokenStream) -> syn::Result<TokenStream> {
    let item: syn::Item = syn::parse2(input)?;
    let (generics, ident) = match &item {
        syn::Item::Enum(item_enum) => (&item_enum.generics, &item_enum.ident),
        syn::Item::Struct(item_struct) => (&item_struct.generics, &item_struct.ident),
        syn::Item::Union(item_union) => (&item_union.generics, &item_union.ident),
        _ => return Err(syn::Error::new(item.span(), "Expected an enum, struct, or union")),
    };

    let crate_path = quote!(spru);
    let private_path = quote!(#crate_path::__private);
    let lookup_ident: syn::Ident = parse_quote!(Lookup);
    let mut lookup_generics = generics.clone();
    lookup_generics.params.push(parse_quote!(#lookup_ident: #crate_path::item::Lookup<Self>));
    lookup_generics.make_where_clause()
        .predicates.push(parse_quote!(Self: for<'de> #private_path::serde::Serialize));
    lookup_generics.make_where_clause()
        .predicates.push(parse_quote!(Self: for<'de> #private_path::serde::Deserialize<'de>));
    let (_impl_generics, type_generics, _where_clause) = generics.split_for_impl();
    let (lookup_impl_generics, _lookup_type_generics, lookup_where_clause) = lookup_generics.split_for_impl();

    Ok(quote! {
        impl #lookup_impl_generics #crate_path::State<#lookup_ident> for #ident #type_generics
        #lookup_where_clause {
            #[doc(hidden)]
            fn apply_state(_index: #crate_path::item::Index, data: &#crate_path::Item<Box<[u8]>>, lookup: &mut #lookup_ident) -> std::result::Result<(), #crate_path::snapshot::ApplyError<#lookup_ident::Error>> {
                #private_path::do_apply_state::<_, Self>(data, lookup)
            }
        }
    })
}