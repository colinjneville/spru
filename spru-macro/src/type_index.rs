use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse2, spanned::Spanned, Expr, ItemEnum, Type};

pub(crate) struct IndexedType<'e> {
    pub(crate) index: Expr,
    pub(crate) ty: &'e Type,
}

pub(crate) struct TypeIndex<'e> {
    pub(crate) item_enum: &'e ItemEnum,
    pub(crate) indexed_types: Vec<IndexedType<'e>>,
}

impl<'e> TypeIndex<'e> {
    pub(crate) fn new(item_enum: &'e ItemEnum) -> syn::Result<Self> {
        let mut indexed_types = vec![];

        let mut next_discriminant = quote!(0u32);
        for variant in &item_enum.variants {
            let mut field_iter = variant.fields.iter();
            let field = field_iter.next()
                .ok_or_else(|| syn::Error::new(variant.span(), "Each variant must have a single field"))?;
            if let Some(second_field) = field_iter.next() {
                return Err(syn::Error::new(second_field.span(), "Each variant must have a single field"));
            }
    
            let discriminant = variant.discriminant.as_ref()
                .map(|(_, d)| d.to_token_stream())
                .unwrap_or(next_discriminant);
    
            next_discriminant = quote!((#discriminant) + 1);
    
            indexed_types.push(IndexedType { index: parse2(discriminant)?, ty: &field.ty });        
        }

        Ok(Self {
            item_enum,
            indexed_types,
        })
    }

    pub fn generate_impls(&self) -> TokenStream {
        let Self {
            item_enum,
            ref indexed_types,
        } = *self;

        let indices = indexed_types.iter()
            .map(|it| &it.index);

        let types = indexed_types.iter()
            .map(|it| &it.ty);

        let ident = &item_enum.ident;
        let (impl_generics, type_generics, where_clause) = item_enum.generics.split_for_impl();
        
        quote! {
            #(
                impl #impl_generics spru::__private::type_index::TypeToU32<#types> for #ident #type_generics
                #where_clause {
                    const N: u32 = #indices;
                }

                impl #impl_generics spru::__private::type_index::U32ToType<{#indices}> for #ident #type_generics
                #where_clause {
                    type Ty = #types;
                }
            )*
        }
    }
}

pub(crate) fn derive_type_index(input: TokenStream) -> syn::Result<TokenStream> {
    let item_enum: ItemEnum = parse2(input)?;
    let type_index = TypeIndex::new(&item_enum)?;

    Ok(type_index.generate_impls())
}