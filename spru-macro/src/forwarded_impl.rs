use proc_macro2::Span;
use quote::quote;
use syn::{parse_quote, punctuated::Punctuated, spanned::Spanned as _, Expr, Field, Generics, Ident, ItemImpl, Pat, Signature, Token, TypeParamBound, Variant};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SingleVariant<'v> {
    pub ident: &'v Ident,
    pub field: &'v Field,
}

impl<'v> SingleVariant<'v> {
    pub fn new(variant: &'v Variant) -> syn::Result<Self> {
        let ident = &variant.ident;
        let mut field_iter = variant.fields.iter();
        let field = match (field_iter.next(), field_iter.next()) {
            (None, None) => Err(syn::Error::new(ident.span(), "Each variant must have a single field")),
            (Some(field), None) => Ok(field),
            (Some(_), Some(field2)) => Err(syn::Error::new(field2.span(), "Each variant must have a single field")),
            _ => unreachable!(),
        }?;
        
        Ok(Self {
            ident,
            field,
        })
    }

    pub fn new_vec(variants: impl IntoIterator<Item = &'v Variant>) -> syn::Result<Vec<Self>> {
        let mut output = vec![];
        for variant in variants {
            output.push(Self::new(variant)?);
        }
        Ok(output)
    }

    pub fn pattern(&self, content_ident: &Ident) -> Pat {
        let ident = self.ident;
        match self.field.ident.as_ref().zip(self.field.colon_token.as_ref()) {
            Some((field_ident, colon)) => parse_quote!(
                #ident { #field_ident #colon #content_ident }
            ),
            None => parse_quote!(#ident(#content_ident)),
        }
    }
}

pub(crate) struct ForwardedImpl {
    pub(crate) impl_trait: Option<syn::Path>,
    pub(crate) functions: Vec<Signature>,
    pub(crate) forward_override: Option<Box<dyn Fn(&Ident, &Punctuated<&Ident, Token![,]>) -> Expr>>,
    pub(crate) field_bounds_override: Option<Punctuated<TypeParamBound, Token![+]>>,
}

impl ForwardedImpl {
    pub(crate) fn generate<'v>(&self, generics: &Generics, ty: &syn::Path, variants: &[SingleVariant<'v>]) -> syn::Result<ItemImpl> {
        let Self {
            impl_trait,
            functions,
            forward_override,
            field_bounds_override,
        } = self;

        let content_ident = Ident::new("__e", Span::call_site());

        let mut fns = quote!();

        let mut generics = generics.clone();

        let bound = if let Some(field_bounds_override) = field_bounds_override {
            Some(quote!(#field_bounds_override))
        } else if let Some(impl_trait) = impl_trait {
            Some(quote!(#impl_trait))
        } else {
            None
        };

        if let Some(bound) = bound {
            for variant in variants {
                let field_type = &variant.field.ty;
                generics.make_where_clause().predicates.push(parse_quote!(#field_type: #bound));
            }
        }

        let impl_trait_for = impl_trait.as_ref()
            .map(|it| quote!(#it for));

        for function in functions {
            let function_ident = &function.ident;
            let function_args: Punctuated<_, Token![,]> = function.inputs.iter()
                .filter_map(|p| match p {
                    syn::FnArg::Receiver(_) => None,
                    syn::FnArg::Typed(pat_type) => {
                        match &*pat_type.pat {
                            Pat::Ident(pat) => Some(&pat.ident),
                            _ => unreachable!(),
                        }
                    }
                }).collect();

            let mut arms = quote!();

            for variant in variants {
                let pattern = variant.pattern(&content_ident);

                let call = if let Some(forward_override) = forward_override {
                    let expr = forward_override(&content_ident, &function_args);
                    quote!(#expr)
                } else if let Some(impl_trait) = &impl_trait {
                    quote!(<#impl_trait>::#function_ident(#content_ident, #function_args))
                } else {
                    quote!(#content_ident.#function_ident(#function_args))
                };
                
                arms = quote! {
                    #arms
                    Self::#pattern => #call,
                }
            }

            fns = quote! {
                #fns

                #function {
                    match self {
                        #arms
                    }
                }
            };
        }

        let (impl_generics, _type_generics, where_clause) = generics.split_for_impl();

        Ok(parse_quote! {
            impl #impl_generics #impl_trait_for #ty
            #where_clause
            {
                #fns
            }
        })
    }
}