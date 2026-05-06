mod script_create;
use script_create::{ScriptCreate, CreateOptions};
mod script_get;
use script_get::{ScriptGet, GetOptions};
mod script_method;
use script_method::{ScriptMethod, MethodOptions};
mod script_set;
use script_set::{ScriptSet, SetOptions};

use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned as _;

const ATTR_GET: &str = "get";
const ATTR_SET: &str = "set";
const ATTR_METHOD: &str = "method";
const ATTR_CREATE: &str = "create";

struct Context {
    type_parameter_state: syn::TypePath,
    type_parameter_action: syn::TypePath,
    parameter_registry: syn::Ident,
    parameter_registration: syn::Ident,
    self_type: syn::Type,
}

trait ScriptImpl {
    fn self_bounds(&self, context: &Context, self_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()>;
    fn registry_bounds(&self, context: &Context, registry_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()>;
    fn action_bounds(&self, context: &Context, action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()>;
    fn other_bounds(&self, context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
        -> syn::Result<()>;
    fn registration(&self, context: &Context, stmts: &mut Vec<syn::Stmt>)
        -> syn::Result<()>;
}

struct Script {
    span: Span,
    self_type: syn::Type,
    partial_ident: Option<syn::Ident>,
    includes: Vec<syn::Ident>,
    items: Vec<ScriptItem>,
    generics: syn::Generics,
}

impl Script {
    #[vacro_report::scope]
    fn build(self, context: &Context) -> syn::Result<TokenStream> {
        let Context {
            type_parameter_state,
            type_parameter_action,
            parameter_registry,
            parameter_registration,
            self_type,
        } = context;

        let mut self_bounds = Default::default();
        self.self_bounds(context, &mut self_bounds)?;

        let mut registry_bounds = Default::default();
        self.registry_bounds(context, &mut registry_bounds)?;

        let mut action_bounds = Default::default();
        self.action_bounds(context, &mut action_bounds)?;

        let mut stmts = vec![];
        self.registration(context, &mut stmts)?;

        let Self {
            span,
            self_type,
            partial_ident,
            includes,
            items,
            mut generics,
        } = self;

        let mut impl_generics = generics.clone();
        let mut trait_generics = syn::Generics::default();

        let impl_type: syn::Type = if let Some(partial_ident) = &partial_ident {
            syn::parse_quote!(#partial_ident::<#self_type>)
        } else {
            self_type.clone()
        };

        trait_generics.params.push(syn::parse_quote_spanned! { span =>
            #type_parameter_state
        });
        trait_generics.params.push(syn::parse_quote_spanned! { span =>
            #type_parameter_action
        });
        trait_generics.params.push(syn::parse_quote_spanned! { span =>
            Registry
        });

        impl_generics.params.push(syn::parse_quote_spanned! { span =>
            #type_parameter_state
        });
        impl_generics.params.push(syn::parse_quote_spanned! { span =>
            #type_parameter_action
        });
        impl_generics.params.push(syn::parse_quote_spanned! { span =>
            Registry
        });

        let impl_generics_where_clause = generics.make_where_clause();
        impl_generics_where_clause.predicates.push(syn::parse_quote_spanned! { span => 
            #self_type: 
                #self_bounds
        });
        impl_generics_where_clause.predicates.push(syn::parse_quote_spanned! { span => 
            #type_parameter_state: 
                spru::State
        });
        impl_generics_where_clause.predicates.push(syn::parse_quote_spanned! { span => 
            #type_parameter_action: 
                spru::Action +
                #action_bounds
        });
        impl_generics_where_clause.predicates.push(syn::parse_quote_spanned! { span => 
            Registry: 
                spru_script::Registry<#type_parameter_state, #type_parameter_action> +
                #registry_bounds
        });

        for item in &items {
            item.other_bounds(context, &mut impl_generics_where_clause.predicates)?;
        }

        for include in &includes {
            let (_, include_generics_type, _) = trait_generics.split_for_impl();
            let mut args: syn::AngleBracketedGenericArguments = syn::parse_quote!(#include_generics_type);
            args.args.push(syn::parse_quote_spanned! { span => 
                Type = #self_type
            });
            generics.make_where_clause().predicates.push(syn::parse_quote_spanned! { span => 
                #include<#self_type>: spru_script::ScriptableType #args
            });
        }

        let (trait_generics_impl, trait_generics_type, trait_generics_where_clause) = trait_generics.split_for_impl();
        let (generics_impl, generics_type, generics_where_clause) = generics.split_for_impl();
        let (impl_generics_impl, impl_generics_type, impl_generics_where_clause) = impl_generics.split_for_impl();
        
        let partial_struct: Option<syn::ItemStruct> = partial_ident.map(|i| syn::parse_quote_spanned! { span => 
            struct #i<T>(T);
        });

        Ok(quote::quote_spanned! { self.span => 
            #partial_struct

            #[allow(unused_parens)]
            impl #impl_generics_impl spru_script::ScriptableType #trait_generics_type for #impl_type
            #generics_where_clause
            {
                type Type = #self_type;
                
                fn register<Storage>(#parameter_registry: &Registry, #parameter_registration: &mut Registry::MemberRegistration<'_, Storage, #self_type>)
                    -> Result<(), Registry::Error> 
                where
                    Storage: spru::item::Storage<State = #type_parameter_state>,
                {
                    #(
                        <#includes<#self_type> as spru_script::ScriptableType #trait_generics_type>::register::<Storage>(
                            #parameter_registry, 
                            #parameter_registration
                        )?;
                    )*
                    #(#stmts)*
                    Ok(())
                }
            }
        })
    }
}

impl ScriptImpl for Script {
    fn self_bounds(&self, context: &Context, self_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()>
    {
        for item in &self.items {
            item.self_bounds(context, self_bounds)?;
        }
        Ok(())
    }

    fn registry_bounds(&self, context: &Context, registry_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>) 
        -> syn::Result<()> 
    {
        for item in &self.items {
            item.registry_bounds(context, registry_bounds)?;
        }
        Ok(())
    }

    fn action_bounds(&self, context: &Context, action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>) 
        -> syn::Result<()> 
    {
        for item in &self.items {
            item.action_bounds(context, action_bounds)?;
        }
        Ok(())
    }

    fn other_bounds(&self, context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
        -> syn::Result<()>
    {
        for item in &self.items {
            item.other_bounds(context, other_bounds)?;
        }
        Ok(())
    }

    fn registration(&self, context: &Context, stmts: &mut Vec<syn::Stmt>)
        -> syn::Result<()> 
    {
        for item in &self.items {
            item.registration(context, stmts)?;
        }
        Ok(())
    }
}

enum ScriptItem {
    Get(ScriptGet),
    Set(ScriptSet),
    Create(ScriptCreate),
    Method(ScriptMethod),
}

impl ScriptImpl for ScriptItem {
    fn self_bounds(&self, context: &Context, self_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()>
    {
        match self {
            ScriptItem::Get(get) => get.self_bounds(context, self_bounds),
            ScriptItem::Set(set) => set.self_bounds(context, self_bounds),
            ScriptItem::Create(create) => create.self_bounds(context, self_bounds),
            ScriptItem::Method(method) => method.self_bounds(context, self_bounds),
        }
    }

    fn registry_bounds(&self, context: &Context, registry_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        match self {
            ScriptItem::Get(get) => get.registry_bounds(context, registry_bounds),
            ScriptItem::Set(set) => set.registry_bounds(context, registry_bounds),
            ScriptItem::Create(create) => create.registry_bounds(context, registry_bounds),
            ScriptItem::Method(method) => method.registry_bounds(context, registry_bounds),
        }
    }

    fn action_bounds(&self, context: &Context, action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        match self {
            ScriptItem::Get(get) => get.action_bounds(context, action_bounds),
            ScriptItem::Set(set) => set.action_bounds(context, action_bounds),
            ScriptItem::Create(create) => create.action_bounds(context, action_bounds),
            ScriptItem::Method(method) => method.action_bounds(context, action_bounds),
        }
    }

    fn other_bounds(&self, context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
        -> syn::Result<()> 
    {
        match self {
            ScriptItem::Get(get) => get.other_bounds(context, other_bounds),
            ScriptItem::Set(set) => set.other_bounds(context, other_bounds),
            ScriptItem::Create(create) => create.other_bounds(context, other_bounds),
            ScriptItem::Method(method) => method.other_bounds(context, other_bounds),
        }
    }

    fn registration(&self, context: &Context, stmts: &mut Vec<syn::Stmt>)
        -> syn::Result<()> 
    {
        match self {
            ScriptItem::Get(get) => get.registration(context, stmts),
            ScriptItem::Set(set) => set.registration(context, stmts),
            ScriptItem::Create(create) => create.registration(context, stmts),
            ScriptItem::Method(method) => method.registration(context, stmts),
        }
    }
}


enum FieldKind {
    Field(syn::Field),
    Virtual(syn::ImplItemFn),
}

impl FieldKind {
    fn field_ident(&self) -> syn::Result<&syn::Ident> {
        const ERR_UNNAMED_FIELD: &str = "Getters and setters must be named fields or functions";

        match self {
            FieldKind::Field(field) => {
                let field_ident = field.ident
                    .as_ref()
                    .ok_or_else(|| syn::Error::new_spanned(field, ERR_UNNAMED_FIELD))?;
                Ok(field_ident)
            }
            FieldKind::Virtual(item_fn) => Ok(&item_fn.sig.ident),
        }
    }
}

enum FieldAttr {
    Get(GetOptions),
    Set(SetOptions),
}

impl FieldAttr {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<(proc_macro2::Span, Self)>> {
        let mut v = vec![];

        for attr in attrs {
            let has_args = !matches!(&attr.meta, syn::Meta::Path(_));
            if attr.meta.path().is_ident("get") {
                let get = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(GetOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Get(get)));
            } else if attr.meta.path().is_ident("set") {
                let set = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(SetOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Set(set)));
            }
        }

        Ok(v)
    }
}

enum FnAttr {
    Get(GetOptions),
    Set(SetOptions),
    Method(MethodOptions),
    Create(CreateOptions),
}

impl FnAttr {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<(proc_macro2::Span, Self)>> {
        let mut v = vec![];

        for attr in attrs {
            let has_args = !matches!(&attr.meta, syn::Meta::Path(_));
            if attr.meta.path().is_ident("get") {
                let get = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(GetOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Get(get)));
            } else if attr.meta.path().is_ident("set") {
                let set = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(SetOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Set(set)));
            } else if attr.meta.path().is_ident("method") {
                let method = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(MethodOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Method(method)));
            } else if attr.meta.path().is_ident("create") {
                let create = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(CreateOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Create(create)));
            }
        }

        Ok(v)
    }
}

vacro_parser::define! { StructOptions:
    #(options*[,]: StructOption {
        Partial: partial = #(partial: syn::Ident),
        Include: include = [#(include*[,]: syn::Ident)],
    })
}

impl StructOptions {
    fn partial(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let StructOption::Partial { partial } = option {
                return Some(partial);
            }
        }
        None
    }

    fn include(&self) -> impl Iterator<Item = &syn::Ident> {
        let mut included = None;
        for option in &self.options {
            if let StructOption::Include { include } = option {
                included = Some(include);
                break;
            }
        }
        
        included.into_iter().flatten()
    }

    #[vacro_report::scope]
    fn build(self, item_struct: &syn::ItemStruct) -> syn::Result<Script> {
        let mut items = vec![];

        for field in &item_struct.fields {
            for (span, attr) in FieldAttr::from_attrs(&field.attrs)? {
                let script_item = match attr {
                    FieldAttr::Get(get_options) => ScriptItem::Get(ScriptGet {
                        span,
                        name_override: get_options.name_override().map(ToString::to_string),
                        field_kind: FieldKind::Field(field.clone()),
                    }),
                    FieldAttr::Set(set_options) => ScriptItem::Set(ScriptSet {
                        span,
                        name_override: set_options.name_override().map(ToString::to_string),
                        field_kind: FieldKind::Field(field.clone()),
                    }),
                };

                items.push(script_item);
            }
        }

        let span = item_struct.span();
        let generics = item_struct.generics.clone();

        let (_, type_generics, _) = generics.split_for_impl();
        let struct_ident = &item_struct.ident;
        let mut self_type = syn::parse_quote!(#struct_ident #type_generics);
        make_turbofish(&mut self_type);

        let partial_ident = self.partial().cloned();
        let includes = self.include().cloned().collect();

        Ok(Script {
            span,
            self_type,
            partial_ident,
            includes,
            items,
            generics,
        })
    }
}

vacro_parser::define! { ImplOptions:
    #(options*[,]: ImplOption {
        Partial: partial = #(partial: syn::Ident),
        Include: include = [#(include*[,]: syn::Ident)],
    })
}

impl ImplOptions {
    fn partial(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let ImplOption::Partial { partial } = option {
                return Some(partial);
            }
        }
        None
    }

    fn include(&self) -> impl Iterator<Item = &syn::Ident> {
        let mut included = None;
        for option in &self.options {
            if let ImplOption::Include { include } = option {
                included = Some(include);
                break;
            }
        }
        
        included.into_iter().flatten()
    }

    fn build(self, item_impl: &syn::ItemImpl) -> syn::Result<Script> {
        let mut items = vec![];
        for sub_item in &item_impl.items {
            if let syn::ImplItem::Fn(impl_item_fn) = sub_item {
                for (span, attr) in FnAttr::from_attrs(&impl_item_fn.attrs)? {
                    let script_item = match attr {
                        FnAttr::Get(get_options) => ScriptItem::Get(ScriptGet {
                            span,
                            name_override: get_options.name_override().map(ToString::to_string),
                            field_kind: FieldKind::Virtual(impl_item_fn.clone()),
                        }),
                        FnAttr::Set(set_options) => ScriptItem::Set(ScriptSet {
                            span,
                            name_override: set_options.name_override().map(ToString::to_string),
                            field_kind: FieldKind::Virtual(impl_item_fn.clone()),
                        }),
                        FnAttr::Method(method_options) => ScriptItem::Method(ScriptMethod { 
                            span, 
                            name_override: method_options.name_override().map(ToString::to_string),
                            method_fn: impl_item_fn.clone(),
                        }),
                        FnAttr::Create(create_options) => ScriptItem::Create(ScriptCreate {
                            span,
                            name_override: create_options.name_override().map(ToString::to_string),
                            create_fn: impl_item_fn.clone(),
                        }),
                    };

                    items.push(script_item);
                }
            }
        }

        let span = item_impl.span();
        let generics = item_impl.generics.clone();

        let mut self_type = (*item_impl.self_ty).clone();
        make_turbofish(&mut self_type);

        let partial_ident = self.partial().cloned();
        let includes = self.include().cloned().collect();

        Ok(Script {
            span,
            self_type,
            partial_ident,
            includes,
            items,
            generics,
        })
    }
}

fn make_turbofish(ty: &mut syn::Type) {
    // Extra angle brackets let us avoid turbofish in most cases, but struct literals
    // (e.g. `X<T> { a, b }` is invalid) are not one of them as far as I can tell.
    // This is probably not bulletproof, but realistically this is always going to
    // be used on TypePath types.
    if let syn::Type::Path(type_path) = ty {
        for segment in &mut type_path.path.segments {
            if let syn::PathArguments::AngleBracketed(angled) = &mut segment.arguments {
                angled.colon2_token = Some(Default::default());
            }
        }
    }
}

#[vacro_report::scope]
pub(crate) fn script_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let item: syn::Item = syn::parse2(item)?;

    let script = match &item {
        syn::Item::Struct(s) => {
            let struct_options: StructOptions = syn::parse2(attr)?;
            Ok(struct_options.build(&s)?)
            
        },
        syn::Item::Impl(i) => {
            let impl_options: ImplOptions = syn::parse2(attr)?;
            Ok(impl_options.build(&i)?)
        },
        _ => Err(syn::Error::new_spanned(&item, "The #[script] attribute can only be applied to structs and impl blocks.")),
    }?;

    let context = Context {
        type_parameter_state: syn::parse_quote! { State },
        type_parameter_action: syn::parse_quote! { Action },
        parameter_registry: syn::parse_quote! { registry },
        parameter_registration: syn::parse_quote! { registration },
        self_type: script.self_type.clone(),
    };

    let script_impl = script.build(&context)?;

    let item = syn::fold::fold_item(&mut HelperFold, item);

    Ok(quote::quote! {
        #item

        #script_impl
    })
}

struct HelperFold;

impl syn::fold::Fold for HelperFold {
    fn fold_attributes(&mut self, mut i: Vec<syn::Attribute>) -> Vec<syn::Attribute> {
        // Strip out our helper attributes
        // https://github.com/rust-lang/rust/issues/65823
        i.retain(|attr| 
            !attr.path().is_ident(ATTR_GET) 
                && !attr.path().is_ident(ATTR_SET) 
                && !attr.path().is_ident(ATTR_METHOD) 
                && !attr.path().is_ident(ATTR_CREATE) 
        );

        // Attributes can't be nested, so we don't need to continue down the AST
        i
    }
}
