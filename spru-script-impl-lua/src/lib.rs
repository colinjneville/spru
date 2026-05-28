#[proc_macro]
pub fn scriptable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    fn internal(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
        let scriptable: spru_script_impl_types::Scriptable = syn::parse2(input)?;
        Ok(proc_macro2::TokenStream::new())
    }

    let (Ok(ts) | Err(ts)) = internal(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}
