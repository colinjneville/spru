use syn::spanned::Spanned;

pub(crate) struct TypeDetails {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
}

impl TypeDetails {
    pub fn parse(item: proc_macro2::TokenStream) -> syn::Result<Self> {
        let item: syn::Item = syn::parse2(item)?;
        match item {
            syn::Item::Enum(item_enum) => Ok(Self {
                ident: item_enum.ident,
                generics: item_enum.generics,
            }),
            syn::Item::Struct(item_struct) => Ok(Self {
                ident: item_struct.ident,
                generics: item_struct.generics,
            }),
            syn::Item::Union(item_union) => Ok(Self {
                ident: item_union.ident,
                generics: item_union.generics,
            }),
            _ => Err(syn::Error::new(item.span(), "Attribute can only be applied to a struct, enum, or union")),
        }
    }
}