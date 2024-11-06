#![allow(non_snake_case)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_quote;

use crate::{util::TypeDetails, ActionArgs};

#[derive(sea_bae::FromAttributes)]
pub struct DestroyArgs {
    // T: syn::Type,
    Undo: syn::Type,
}

impl From<(TypeDetails, DestroyArgs)> for ActionArgs {
    fn from((type_details, value): (TypeDetails, DestroyArgs)) -> Self {
        let TypeDetails {
            ident,
            generics,
            ..
        } = type_details;
        let DestroyArgs {
            // T: t,
            Undo: undo,
        } = value;

        ActionArgs {
            ident,
            generics,
            // t,
            adapter: parse_quote!(::spru::action::adapter::Destroy),
            error: None,
            undo,
        }
    }
}

pub(crate) fn fn_destroy(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let type_details = TypeDetails::parse(item.clone())?;
    let pseudo_attrs = parse_quote! {
        #[destroy_args(#attr)]
    };
    
    let base = ActionArgs::from((type_details, DestroyArgs::from_attributes(&[pseudo_attrs])?)).into_token_stream();

    Ok(quote! { 
        #item
        #base
    })
}