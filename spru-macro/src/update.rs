use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

use crate::ActionImpl;

pub(crate) fn fn_update(item: TokenStream) -> syn::Result<TokenStream> {
    let item: syn::Item = syn::parse2(item)?;

    let action_impl = ActionImpl::new(&item, parse_quote!(Update), parse_quote!(update))?;

    Ok(quote! {
        #action_impl
    })
}
