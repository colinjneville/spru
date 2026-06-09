#[proc_macro]
pub fn scriptable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    fn internal(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
        // Parse input just to catch mistakes early.
        let _scriptable: spru_script_base_macro::Scriptable = syn::parse2(input)?;

        Err(syn::Error::new(proc_macro2::Span::call_site(), "stub"))?;

        Ok(proc_macro2::TokenStream::new())
    }

    let (Ok(ts) | Err(ts)) = internal(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}
