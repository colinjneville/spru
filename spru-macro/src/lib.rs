mod action_catalog;
mod create;
mod destroy;
mod update;
mod from_infallible;
mod payload_variant;
mod item_catalog;
mod type_index;
mod util;

pub(crate) struct ActionArgs {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    // pub t: syn::Type,
    pub adapter: syn::Type,
    pub error: Option<syn::Type>,
    pub undo: syn::Type,
}

impl quote::ToTokens for ActionArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use syn::parse_quote;

        let Self {
            ident,
            generics,
            // t,
            adapter,
            error,
            undo,
        } = self;
        
        let default_error = parse_quote!(::std::convert::Infallible);
        let error = error.as_ref().unwrap_or(&default_error);        

        let mut base_generics = generics.clone();
        base_generics.make_where_clause().predicates.push(parse_quote!(Self: ::std::clone::Clone + ::spru::Serial));

        let mut base2_generics = generics.clone();
        base2_generics.make_where_clause().predicates.push(parse_quote!(Self: ::spru::action::Base));

        let mut entry_generics = generics.clone();
        entry_generics.params.push(parse_quote!(Lookup: ::spru::item::Lookup));
        entry_generics.make_where_clause().predicates.push(parse_quote!(Self: ::spru::Action));
        entry_generics.make_where_clause().predicates.push(parse_quote!(Lookup: ::spru::item::lookup::OfTypeMut<<Self as ::spru::Action>::T>));

        let (base_impl_generics, base_type_generics, base_where_clause) = base_generics.split_for_impl();
        let (base2_impl_generics, base2_type_generics, base2_where_clause) = base2_generics.split_for_impl();
        let (entry_impl_generics, _entry_type_generics, entry_where_clause) = entry_generics.split_for_impl();

        quote::quote! {
            impl #base_impl_generics ::spru::action::Base for #ident #base_type_generics
            #base_where_clause {        
                type Error = #error;
                type Undo = #undo;
            }

            impl #base2_impl_generics ::spru::action::Base2 for #ident #base2_type_generics
            #base2_where_clause {
                // type T = #t;
                type Adapter = #adapter;
            }

            impl #entry_impl_generics ::spru::action::catalog::Entry<Lookup> for #ident #base_type_generics
            #entry_where_clause {
                fn __apply_entry(&self, mut data: ::spru::action::adapter::Data<'_, Lookup>) -> Result<Option<Self::Undo>, ::spru::action::catalog::Error<<Lookup as ::spru::item::Lookup>::Error, Self::Error>>
                // where Lookup: 'l
                {
                    use ::spru::action::Adapter as _;
                    use ::spru::Action as _;
                    
                    let input = <Self as spru::action::Base2>::Adapter::input(&mut data)?;
                    let output = self
                        .apply(input)
                        .map_err(::spru::action::catalog::Error::Action)?
                        .into();
                    let undo = <Self as spru::action::Base2>::Adapter::output(&mut data, output)?;
                    Ok(undo)
                }
            }
        }.to_tokens(tokens);
    }
}

#[proc_macro_derive(ActionCatalog, attributes(catalog))]
pub fn action_catalog(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = action_catalog::derive_action_catalog(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_derive(ItemCatalog, attributes(catalog))]
pub fn item_catalog(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = item_catalog::derive_item_catalog(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_derive(TypeIndex)]
pub fn type_index(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = type_index::derive_type_index(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_attribute]
pub fn create(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = create::fn_create(attr.into(), item.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_attribute]
pub fn destroy(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = destroy::fn_destroy(attr.into(), item.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_attribute]
pub fn update(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = update::fn_update(attr.into(), item.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_derive(FromInfallible)]
pub fn from_infallible(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = from_infallible::derive_from_infallible(input.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}

#[proc_macro_attribute]
pub fn payload_variant(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (Ok(ts) | Err(ts)) = payload_variant::payload_variant_attr(attr.into(), item.into())
        .map_err(syn::Error::into_compile_error);
    ts.into()
}
