#![allow(non_snake_case)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_quote;

use crate::{util::TypeDetails, ActionImpl};

pub(crate) fn fn_create(item: TokenStream) -> syn::Result<TokenStream> {
    let item: syn::Item = syn::parse2(item)?;
    
    let action_impl = ActionImpl::new(&item, parse_quote!(Create), parse_quote!(create))?;

    Ok(quote! {
        #action_impl
    })
}