use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse2, parse_quote, ItemEnum};

pub(crate) fn derive_item_catalog(input: TokenStream) -> syn::Result<TokenStream> {
    let item_enum: ItemEnum = parse2(input)?;

    let type_index = crate::type_index::TypeIndex::new(&item_enum)?;
    let type_index_impls = type_index.generate_impls();

    let param_index = quote!(index);
    let param_item = quote!(item);
    let param_lookup = quote!(lookup);
    
    let ty_param_lookup = quote!(Lookup);

    let mut generics_extra = item_enum.generics.clone();
    generics_extra.params.push(parse2(ty_param_lookup.clone())?);
    let where_clause_extra = generics_extra.make_where_clause();

    where_clause_extra.predicates.push(parse_quote!(#ty_param_lookup: spru::item::Lookup));

    let mut const_vals = quote!();
    let mut arms = quote!();

    for (i, indexed_type) in type_index.indexed_types.into_iter().enumerate() {
        let crate::type_index::IndexedType {
            index,
            ty,
        } = indexed_type;

        let const_ident = format_ident!("__INDEX{}", i);

        const_vals = quote! {
            #const_vals
            const #const_ident: u32 = #index;
        };

        arms = quote! {
            #arms
            #const_ident => spru::__private::do_apply_item::<_, #ty>(#param_item, #param_lookup),
        };

        where_clause_extra.predicates.push(parse_quote!(#ty_param_lookup: spru::item::lookup::OfTypeMut<#ty>));
    }

    let ident = &item_enum.ident;
    let (_, type_generics, _) = item_enum.generics.split_for_impl();
    let (impl_generics_extra, _, where_clause_extra) = generics_extra.split_for_impl();

    Ok(quote! {
        #type_index_impls

        impl #impl_generics_extra spru::item::Catalog<#ty_param_lookup> for #ident #type_generics
        #where_clause_extra {
            fn apply_item(#param_index: spru::item::catalog::Index, #param_item: &spru::Item<Box<[u8]>>, #param_lookup: &mut #ty_param_lookup) -> ::std::result::Result<(), spru::snapshot::ApplyError<#ty_param_lookup::Error>> {
                #const_vals
                match #param_index {
                    #arms
                    _ => unreachable!("Invalid type index {}", #param_index),
                }
            }
        }
    })
}