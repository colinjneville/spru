mod script;
mod scriptable_path;

#[proc_macro_attribute]
pub fn script(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(token_stream) | Err(token_stream)) = script::script_impl(attr.into(), item.into())
        .map_err(syn::Error::into_compile_error);
    token_stream.into()
}

#[proc_macro]
pub fn scriptable_path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path = syn::parse_macro_input!(input as syn::Path);
    let scriptable_path = scriptable_path::ScriptablePath::parse(&path);
    quote::quote! {
        #scriptable_path
    }.into()
}

