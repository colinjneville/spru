pub(crate) fn registration_ident() -> syn::Ident {
    syn::Ident::new("__registration1", proc_macro2::Span::call_site())
}