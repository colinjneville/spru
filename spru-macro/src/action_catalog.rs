use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse2, parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, Token, ItemEnum, MetaNameValue};

pub(crate) fn derive_action_catalog(input: TokenStream) -> syn::Result<TokenStream> {
    let item_enum: ItemEnum = parse2(input)?;

    let mut error_path: Option<syn::Path> = None;
    let mut new_error_variants = None;
    // let mut added_bounds = vec![];
    for attribute in &item_enum.attrs {
        let attribute_path = attribute.meta.path();

        if attribute_path.is_ident("catalog") {
            let args = attribute.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)?;

            for arg in args {
                let old_error_path = if arg.path.is_ident("error") {
                    error_path.replace(parse2(arg.value.into_token_stream())?)
                } else if arg.path.is_ident("new_error") {
                    let new_error_path: syn::Path = parse2(arg.value.into_token_stream())?;
                    if new_error_path.get_ident().is_some() {
                        return Err(syn::Error::new(new_error_path.span(), 
                            "#[action::catalog(new_error = ...)] must contain the full path to the current module, followed by the desired enum identifier.\n
                            E.g. `#[action::catalog(new_error = crate::my_actions::Error)]` to create an enum named `Error` if this enum resides in `crate::my_actions`."
                        ))
                    }

                    new_error_variants = Some(quote!());
                    error_path.replace(new_error_path)
                } else if arg.path.is_ident("bound") {
                    todo!()
                } else {
                    return Err(syn::Error::new(arg.path.span(), "Invalid parameter"));
                };

                if old_error_path.is_some() {
                    return Err(syn::Error::new(attribute.span(), "Exactly one action::Catalog error must be declared."));
                }
            }
        }
    }

    let item_ident = &item_enum.ident;
    // TODO generics processing is currently an unitelligible (and probably incorrect) mess
    let item_generics = &item_enum.generics;

    let (impl_generics, _ty_generics, where_clause) = item_generics.split_for_impl();

    let mut base_generics = item_generics.clone();
    base_generics.make_where_clause().predicates.push(parse_quote!(Self: ::std::clone::Clone + ::spru::Serial));

    let (base_impl_generics, base_ty_generics, base_where_clause) = base_generics.split_for_impl();

    let mut action_generics = item_generics.clone();
    action_generics.params.push(parse_quote!(Lookup: ::spru::item::Lookup));
    let mut action_catalog_generics = action_generics.clone();
    action_catalog_generics.make_where_clause().predicates.push(parse_quote!(Self: ::spru::action::Base));

    action_generics.make_where_clause().predicates.push(parse_quote!(Self: ::spru::action::Catalog<Lookup, Undo = Self>));

    let mut variants = vec![];
    let mut noop_ident = None;
    for variant in &item_enum.variants {
        let mut named_iter;
        let mut unnamed_iter;

        let iter = match &variant.fields {
            syn::Fields::Named(fs) => {
                named_iter = fs.named.iter();
                Some(&mut named_iter)
            }
            syn::Fields::Unnamed(fs) => {
                unnamed_iter = fs.unnamed.iter();
                Some(&mut unnamed_iter)
            }
            syn::Fields::Unit => {
                if noop_ident.replace(&variant.ident).is_some() {
                    return Err(syn::Error::new(variant.span(), "Only one fieldless variant is allowed."));
                }
                None
            },
        };

        if let Some(iter) = iter {
            let first = iter.next().unwrap();
            if let Some(field) = iter.next() {
                return Err(syn::Error::new(field.span(), "Variants may have at most one field."));
            }
            
            variants.push((&variant.ident, first));
        }
    }

    let error_path = error_path.unwrap_or_else(|| parse_quote!(::std::convert::Infallible));
    // let Some(error_path) = error_path else {
    //     return Err(syn::Error::new(item_enum.span(), "#[catalog(error = ...)] or #[catalog(new_error = ...)] is required."));
    // };

    let mut arms = quote!();

    for (ident, field) in variants {
        let span = ident.span();

        let field_tokens = if let Some(field_ident) = &field.ident {
            quote! {
                { #field_ident: action }
            }
        } else {
            quote! {
                (action)
            }
        };
        arms = quote_spanned! { span =>
            #arms
            Self::#ident #field_tokens => Ok(
                action.__apply_entry(data)
                    .map_err(::spru::action::catalog::Error::map_action)?
                    .map(::std::convert::Into::into)
            ),
        };
        if let Some(new_error_variants) = new_error_variants.as_mut() {
            // TODO Technically `Self` does not work correctly, but is there ever a good reason to use
            // `Self` in an `action::Catalog` aggregate enum?
            let field_ty = &field.ty;
            *new_error_variants = quote_spanned! { span =>
                #new_error_variants
                #ident(<#field_ty as ::spru::Action>::Error),
            };
        }

        let field_ty = &field.ty;
        let predicates = &mut action_catalog_generics.make_where_clause().predicates;
        predicates.push(parse_quote_spanned! { span =>
            #field_ty: ::spru::action::catalog::Entry<
                Lookup,
                Undo: ::std::convert::Into<Self>, 
                Error: ::std::convert::Into<Self::Error>,
            >
        });
    }

    if let Some(noop_ident) = noop_ident {
        let span = noop_ident.span();
        arms = quote_spanned! { span =>
            #arms
            Self::#noop_ident => Ok(Self::#noop_ident),
        };
    }

    let new_error = if let Some(new_error_variants) = new_error_variants {
        let new_error_ident = &error_path.segments.last().unwrap().ident;
        let span = error_path.span();
        Some(quote_spanned! { span =>
            #[derive(::std::fmt::Debug, ::std::clone::Clone)]
            #[::spru::__private::telety::telety(#error_path, telety_path = "::spru::__private::telety")]
            #[::spru::__private::amass::amass]
            pub enum #new_error_ident #impl_generics
            #where_clause {
                #new_error_variants
            }
        })
    } else {
        None
    };

    let (action_catalog_impl_generics, _action_catalog_ty_generics, action_catalog_where_clause) = action_catalog_generics.split_for_impl();

    let (action_impl_generics, _action_ty_generics, action_where_clause) = action_generics.split_for_impl();

    Ok(quote! {
        impl #base_impl_generics ::spru::action::Base for #item_ident #base_ty_generics
        #base_where_clause {
            type Error = #error_path;
            type Undo = Self;
        }

        impl #action_catalog_impl_generics ::spru::action::Catalog<Lookup> for #item_ident #base_ty_generics
        #action_catalog_where_clause
        {
            fn apply(&self, data: ::spru::action::adapter::Data<Lookup>) -> Result<::std::option::Option<Self>, ::spru::action::catalog::Error<lookup::Error, Self::Error>> {
                use ::spru::action::catalog::Entry as _;
                match self {
                    #arms
                }
            }
        }

        impl #action_impl_generics ::spru::action::catalog::Entry<Lookup> for #item_ident #base_ty_generics
        #action_where_clause {
            // type Adapter = ::spru::action::adapter::Passthrough;
            // type Error = ::spru::action::catalog::Error<lookup::Error, <Self as ::spru::action::Catalog>::Error>;
        
            // type Undo = Self;

            fn __apply_entry(&self, mut data: ::spru::action::adapter::Data<'_, Lookup>) -> Result<Option<Self::Undo>, ::spru::action::catalog::Error<<Lookup as ::spru::item::Lookup>::Error, Self::Error>>
            // where 
            //     Lookup: 'l,
            //     Self::Adapter: ::spru::action::Adapter<Lookup>,
            {
                ::spru::action::Catalog::apply(self, data)
            }
        }

        #new_error
    })
}