#![allow(non_snake_case)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_quote;

use crate::{util::TypeDetails, ActionArgs};

#[derive(sea_bae::FromAttributes)]
pub struct UpdateArgs {
    // T: syn::Type,
    Undo: Option<syn::Type>,
    Error: Option<syn::Type>,
}

impl From<(TypeDetails, UpdateArgs)> for ActionArgs {
    fn from((type_details, value): (TypeDetails, UpdateArgs)) -> Self {
        let TypeDetails {
            ident,
            generics,
            ..
        } = type_details;
        let UpdateArgs {
            // T: t,
            Undo: undo,
            Error: error,
        } = value;

        let undo = undo.unwrap_or_else(|| parse_quote!(Self));

        ActionArgs {
            ident,
            generics,
            // t,
            adapter: parse_quote!(::spru::action::adapter::Update),
            error,
            undo,
        }
    }
}

pub(crate) fn fn_update(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let type_details = TypeDetails::parse(item.clone())?;
    let pseudo_attrs = parse_quote! {
        #[update_args(#attr)]
    };
    
    let base = ActionArgs::from((type_details, UpdateArgs::from_attributes(&[pseudo_attrs])?)).into_token_stream();

    Ok(quote! { 
        #item
        #base
    })
}