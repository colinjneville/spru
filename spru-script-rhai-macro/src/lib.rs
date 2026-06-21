// mod scriptable;
mod impl_dynamic_fn;

#[proc_macro]
pub fn impl_dynamic_fn(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = impl_dynamic_fn::impl_dynamic_fn(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

