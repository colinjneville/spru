use syn::spanned::Spanned as _;

mod create;
mod destroy;
mod from_infallible;
mod update;

pub(crate) struct ActionImpl<'i> {
    pub ident: &'i syn::Ident,
    pub generics: &'i syn::Generics,
    pub trait_ident: syn::Ident,
    pub func_ident: syn::Ident,
}

impl<'i> ActionImpl<'i> {
    pub fn new(
        item: &'i syn::Item,
        trait_ident: syn::Ident,
        func_ident: syn::Ident,
    ) -> syn::Result<Self> {
        let (ident, generics) = match item {
            syn::Item::Enum(item_enum) => (&item_enum.ident, &item_enum.generics),
            syn::Item::Struct(item_struct) => (&item_struct.ident, &item_struct.generics),
            syn::Item::Union(item_union) => (&item_union.ident, &item_union.generics),
            _ => {
                return Err(syn::Error::new(
                    item.span(),
                    "Attribute can only be applied to a struct, enum, or union",
                ));
            }
        };

        Ok(Self {
            ident,
            generics,
            func_ident,
            trait_ident,
        })
    }
}

impl<'i> quote::ToTokens for ActionImpl<'i> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use syn::parse_quote;

        let Self {
            ident,
            generics,
            ref func_ident,
            ref trait_ident,
        } = *self;

        let trait_path: syn::Path = parse_quote!(::spru::action::#trait_ident);

        let mut generics = generics.clone();
        generics.make_where_clause().predicates.push(parse_quote!(
            Self: #trait_path
        ));

        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

        quote::quote! {
            impl #impl_generics spru::action::SubAction for #ident #type_generics
                #where_clause
            {
                type Undo = <Self as #trait_path>::Undo;
                type T = <Self as #trait_path>::T;

                fn apply<Lookup>(&self, context: ::spru::action::Context<'_, Lookup>)
                    -> ::spru::action::Result<Option<Self::Undo>>
                where
                    Lookup: ::spru::item::Lookup,
                    Self::T: spru::item::lookup::Lookupable<Lookup::State>,
                {
                    context.#func_ident(self)
                }
            }
        }
        .to_tokens(tokens);
    }
}

#[proc_macro_derive(Create)]
pub fn create(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = create::fn_create(item.into()).map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_derive(Destroy)]
pub fn destroy(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) =
        destroy::fn_destroy(item.into()).map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_derive(Update)]
pub fn update(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = update::fn_update(item.into()).map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_derive(FromInfallible)]
pub fn from_infallible(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = from_infallible::derive_from_infallible(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}
