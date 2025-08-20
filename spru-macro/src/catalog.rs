use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse::Parse, parse2, parse_quote, spanned::Spanned as _, Attribute, Generics, Ident, ImplItem, ItemEnum, Token, Type, Variant, WherePredicate};

use crate::forwarded_impl::{ForwardedImpl, SingleVariant};

struct AssociatedType {
    assoc: Ident,
    kind: AssociatedTypeKind,
}

impl Parse for AssociatedType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            assoc: input.parse()?,
            kind: input.parse()?,
        })
    }
}

enum AssociatedTypeKind {
    Existing(ExistingType),
    Generated(GeneratedType),
}

impl Parse for AssociatedTypeKind {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![=]) {
            Ok(Self::Existing(ExistingType::parse(input)?))
        } else if lookahead.peek(Token![as]) {
            Ok(Self::Generated(GeneratedType::parse(input)?))
        } else {
            Err(lookahead.error())
        }
    }
}

struct ExistingType {
    _eq: Token![=],
    ty: Type,
}

impl Parse for ExistingType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _eq: input.parse()?,
            ty: input.parse()?,
        })
    }
}

struct GeneratedType {
    _as: Token![as],
    attrs: Vec<Attribute>,
    ident: Ident,
}

impl Parse for GeneratedType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _as: input.parse()?,
            attrs: Attribute::parse_outer(input)?,
            ident: input.parse()?,
        })
    }
}

#[derive(Default)]
struct Settings {
    data: Option<AssociatedTypeKind>,
    error: Option<AssociatedTypeKind>,
}

impl Settings {
    fn new(item_enum: &ItemEnum) -> syn::Result<Self> {
        let mut settings = Self::default();

        for attribute in &item_enum.attrs {
            let attribute_path = attribute.meta.path();
    
            if attribute_path.is_ident("catalog") {
                let AssociatedType {
                    assoc,
                    kind,
                } = attribute.parse_args_with(AssociatedType::parse)?;
                
                if assoc == "Error" {
                    if let Some(_) = settings.error.replace(kind) {
                        return Err(syn::Error::new(assoc.span(), format!("A maximum of 1 '{}' catalog attribute is allowed", assoc)));
                    }
                } else if assoc == "Data" {
                    if let Some(_) = settings.data.replace(kind) {
                        return Err(syn::Error::new(assoc.span(), format!("A maximum of 1 '{}' catalog attribute is allowed", assoc)));
                    }
                } else {
                    return Err(syn::Error::new(assoc.span(), "Unknown catalog attribute"));
                }
            }
        }

        Ok(settings)
    }
}

pub(crate) fn derive_catalog(input: TokenStream) -> syn::Result<TokenStream> {
    let item_enum: ItemEnum = parse2(input)?;
    let item_ident = &item_enum.ident;
    let generics = &item_enum.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let vis = &item_enum.vis;

    let lookup_ident = Ident::new("Lookup", Span::call_site());

    let mut lookup_generics = item_enum.generics.clone();
    lookup_generics.params.push(parse_quote!(#lookup_ident: ::spru::item::Lookup));
    let (lookup_impl_generics, _lookup_type_generics, lookup_where_clause) = lookup_generics.split_for_impl();

    let Settings {
        data,
        error,
    } = Settings::new(&item_enum)?;
    
    let single_variants = SingleVariant::new_vec(&item_enum.variants)?;

    let Some(first_variant) = single_variants.first() else {
        return Err(syn::Error::new(item_enum.span(), "Enum must have at least one variant"));
    };

    let mut entry_base_generics = generics.clone();
    entry_base_generics.make_where_clause().predicates.push(parse_quote!(Self: ::spru::catalog::Base));
    let mut entry_generics = lookup_generics.clone();
    entry_generics.make_where_clause().predicates.push(parse_quote!(Self: ::spru::Catalog::<#lookup_ident>));
    let data = data.unwrap_or_else(|| {
        let ty = &first_variant.field.ty;
        entry_base_generics.make_where_clause().predicates.push(parse_quote!(#ty: ::spru::catalog::entry::Base));
        entry_generics.make_where_clause().predicates.push(parse_quote!(#ty: ::spru::catalog::entry::Base));

        AssociatedTypeKind::Existing(parse_quote!(= <#ty as ::spru::catalog::entry::Base>::Out))
    });
    let error = error.unwrap_or_else(|| AssociatedTypeKind::Existing(parse_quote!(= ::std::convert::Infallible)));

    let catalog_base_impl = ForwardedImpl { 
        impl_trait: Some(parse_quote!(::spru::catalog::Base)), 
        functions: vec![],
        forward_override: None,
        field_bounds_override: Some(parse_quote!(::spru::catalog::entry::Base)), 
    };

    let catalog_impl = {
        let lookup_ident_clone = lookup_ident.clone();
        ForwardedImpl { 
            impl_trait: Some(parse_quote!(::spru::Catalog<#lookup_ident>)), 
            functions: vec![
                parse_quote!(fn apply(&self, context: ::spru::catalog::Context<#lookup_ident>) -> ::std::result::Result<::std::option::Option<Self>, ::spru::action::Error<#lookup_ident::Error, Self::Error>>),
            ],
            forward_override: Some(Box::new(move |ident, args| parse_quote!(
                ::spru::catalog::Entry::<#lookup_ident_clone>::entry(#ident, #args)
                    .map(|o| o.map(Into::into))
                    .map_err(::spru::action::Error::map_action)
            ))),
            field_bounds_override: Some(parse_quote!(::spru::catalog::Entry<#lookup_ident, Out: Into<Self>, Error: Into<Self::Error>>)), 
        }
    };

    let mut output = quote!();
    
    let error_type = match &error {
        AssociatedTypeKind::Generated(generated_type) => {
            let GeneratedType {
                _as,
                attrs: generated_type_attr,
                ident: generated_type_ident,
            } = generated_type;

            let mut error_generics = item_enum.generics.clone();
            let predicates = &mut error_generics.make_where_clause().predicates;
            for &SingleVariant { field, .. } in &single_variants {
                let field_type = &field.ty;
                predicates.push(parse_quote!(
                    #field_type: ::spru::catalog::entry::Base
                ));
            }
            
            let ty: syn::Path = parse_quote!(#generated_type_ident #type_generics);

            let forwarded_impls = vec![
                ForwardedImpl { 
                    impl_trait: Some(parse_quote!(::std::fmt::Debug)), 
                    functions: vec![
                        parse_quote!(fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result),
                    ], 
                    field_bounds_override: None,
                    forward_override: None,
                },
                ForwardedImpl { 
                    impl_trait: Some(parse_quote!(::std::fmt::Display)), 
                    functions: vec![
                        parse_quote!(fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result),
                    ], 
                    field_bounds_override: None,
                    forward_override: None,
                },
                ForwardedImpl { 
                    impl_trait: Some(parse_quote!(::std::error::Error)), 
                    functions: vec![
                        parse_quote!(fn source(&self) -> ::std::option::Option<&(dyn ::std::error::Error + 'static)>),
                    ], 
                    field_bounds_override: None,
                    forward_override: None,
                },
            ];

            let error_variants: Vec<Variant> = single_variants.iter().map(|&SingleVariant { ident, field }| 
                parse_quote!(#ident(<#field as ::spru::catalog::entry::Base>::Error))
            ).collect();
            let error_single_variants = SingleVariant::new_vec(&error_variants)?;

            // TODO This doesn't really work because often these traits won't 
            // be implemented (e.g. Infallible won't implement Error, etc.)
            let mut impls = quote!();
            // for forwarded_impl in forwarded_impls {
            //     let im = forwarded_impl.generate(&item_enum.generics, &ty, &error_single_variants)?;
            //     impls = quote! {
            //         #impls
            //         #im
            //     }
            // }

            let mut arms = quote!();
            for &SingleVariant { ident, field } in &single_variants {
                arms = quote!(
                    #arms
                    #ident(<#field as ::spru::catalog::entry::Base>::Error),
                );
            }

            let ident = &generated_type.ident;

            let (impl_error_generics, _type_error_generics, error_where_clause) = error_generics.split_for_impl();

            output = quote! {
                #output

                #(#generated_type_attr)*
                #vis enum #ident #impl_error_generics
                #error_where_clause {
                    #arms
                }

                #impls
            };

            quote!(#ident #type_generics)
        }
        AssociatedTypeKind::Existing(existing_type) => existing_type.ty.to_token_stream(),
    };

    let data_type = match &data {
        AssociatedTypeKind::Generated(generated_type) => {
            let mut arms = quote!();
            for &SingleVariant { ident, field } in &single_variants {
                arms = quote!(
                    #arms
                    #ident(<#field as ::spru::catalog::Base>::Data),
                );
            }

            let ident = &generated_type.ident;

            output = quote! {
                #output

                #[derive(::std::fmt::Debug)]
                // TODO needs amass
                #vis enum #ident #impl_generics
                #where_clause {
                    #arms
                }
            };

            quote!(#ident #type_generics)
        }
        AssociatedTypeKind::Existing(existing_type) => existing_type.ty.to_token_stream(),
    };

    let ty: syn::Path = parse_quote!(#item_ident #type_generics);
    
    let mut catalog_base_impl = catalog_base_impl.generate(&item_enum.generics, &ty, &*single_variants)?;
    catalog_base_impl.items.push(
        ImplItem::Type(parse_quote!(type Error = #error_type;))
    );
    catalog_base_impl.items.push(
        ImplItem::Type(parse_quote!(type Data = #data_type;))
    );
    let catalog_impl = catalog_impl.generate(&lookup_generics, &ty, &*single_variants)?;

    let (entry_impl_generics, entry_type_generics, entry_where_clause) = entry_base_generics.split_for_impl();
    let (entry_lookup_impl_generics, _entry_lookup_type_generics, entry_lookup_where_clause) = entry_generics.split_for_impl();

    output = quote! {
        #output

        #catalog_base_impl

        #catalog_impl

        impl #entry_impl_generics ::spru::catalog::entry::Base for #item_ident #entry_type_generics 
        #entry_where_clause {
            type Out = Self;
            type Error = <Self as ::spru::catalog::Base>::Error;
        }

        impl #entry_lookup_impl_generics ::spru::catalog::Entry<#lookup_ident> for #item_ident #entry_type_generics 
        #entry_lookup_where_clause {
            fn entry(&self, context: ::spru::catalog::Context<#lookup_ident>) -> ::std::result::Result<::std::option::Option<Self>, ::spru::action::Error<#lookup_ident::Error, Self::Error>> {
                ::spru::Catalog::apply(self, context)
            }
        }
    };

    Ok(output)
}