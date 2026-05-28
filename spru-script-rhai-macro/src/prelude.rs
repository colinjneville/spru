use syn::spanned::Spanned as _;


pub(crate) fn prelude(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let span = input.span();
    let rhai_expr: syn::Expr = syn::parse2(input)?;

    let registration_ident = crate::common::registration_ident();

    Ok(quote::quote_spanned! { span => 
        use spru_script_rhai::{
            RegisterTypeNoop as _, RegisterTypeStd as _, 
            RegisterTypeGetNoop as _, RegisterTypeGetStd as _, 
            RegisterTypeMethodNoop as _, RegisterTypeMethodStd as _, 
            RegisterTypeFunctionNoop as _, RegisterFunctionStd as _, 
            RegisterStateNoop as _, RegisterStateStd as _, 
            RegisterStateGetNoop as _, RegisterStateGetStd as _, 
            RegisterStateSetNoop as _, RegisterStateSetStd as _, 
            RegisterStateMethodNoop as _, RegisterStateMethodStd as _, 
            RegisterStateFunctionNoop as _, RegisterStateFunctionStd as _, 
            RegisterStateCreateNoop as _, RegisterStateCreateStd as _, 
        };

        let #registration_ident = spru_script::Registration1::new(#rhai_expr);
    })
}