use syn::spanned::Spanned as _;


pub(crate) fn postlude(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let span = input.span();

    let registration_ident = crate::common::registration_ident();

    Ok(quote::quote_spanned! {span => 
        #registration_ident.apply();
    })
}