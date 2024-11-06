#![allow(non_snake_case)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_quote;

use crate::{util::TypeDetails, ActionArgs};

#[derive(sea_bae::FromAttributes)]
pub struct CreateArgs {
    // T: syn::Type,
    Undo: syn::Type,
    Error: Option<syn::Type>,
}

impl From<(TypeDetails, CreateArgs)> for ActionArgs {
    fn from((type_details, value): (TypeDetails, CreateArgs)) -> Self {
        let TypeDetails {
            ident,
            generics,
            ..
        } = type_details;
        let CreateArgs {
            // T: t,
            Undo: undo,
            Error: error,
        } = value;

        ActionArgs {
            ident,
            generics,
            // t,
            adapter: parse_quote!(::spru::action::adapter::Create),
            error,
            undo,
        }
    }
}

pub(crate) fn fn_create(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let type_details = TypeDetails::parse(item.clone())?;

    let pseudo_attrs = parse_quote! {
        #[create_args(#attr)]
    };
    
    let base = ActionArgs::from((type_details, CreateArgs::from_attributes(&[pseudo_attrs])?)).into_token_stream();

    Ok(quote! { 
        #item
        #base
    })
}