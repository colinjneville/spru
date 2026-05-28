pub(crate) mod common;
mod postlude;
mod prelude;
mod scriptable;

#[proc_macro]
pub fn postlude(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = postlude::postlude(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro]
pub fn prelude(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = prelude::prelude(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro]
pub fn scriptable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = scriptable::scriptable(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

