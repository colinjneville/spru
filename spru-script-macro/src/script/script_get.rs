use proc_macro2::{Span, TokenStream};

use super::{Context, FieldKind};


pub(crate) struct ScriptGet {
    pub span: Span,
    pub name_override: Option<String>,
    pub field_kind: FieldKind,
    pub wrap: bool,
}

impl ScriptGet {
    pub fn field_type(&self) -> syn::Result<syn::Type> {
        const ERR_NEEDS_RETURN: &str = "A getter function must return the type of the virtual field";

        let field_type = match &self.field_kind {
            FieldKind::Field(field) => Ok(&field.ty),
            FieldKind::Virtual(item_fn) => match &item_fn.sig.output {
                syn::ReturnType::Default => Err(syn::Error::new_spanned(&item_fn, ERR_NEEDS_RETURN)),
                syn::ReturnType::Type(_rarrow, ty) => Ok(&**ty),
            },
        }?;

        let field_type = if self.wrap {
            syn::parse_quote_spanned! { self.span => 
                spru_script::Wrap::<#field_type>
            }
        } else {
            field_type.clone()
        };

        Ok(field_type)
    }
}

impl super::ScriptImpl for ScriptGet {
    fn self_bounds(&self, _context: &Context, self_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()>
    {
        Ok(())
    }

    #[vacro_report::scope]
    fn registry_bounds(&self, context: &Context, registry_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        let Context {
            type_parameter_state,
            type_parameter_action,
            self_type,
            is_state,
            ..
        } = context;

        let registry_trait_str = if *is_state { "RegistryStateGet" } else { "RegistryTypeGet" };
        let registry_trait = syn::Ident::new(registry_trait_str, self.span);

        let field_type = self.field_type()?;
        let bound = syn::parse_quote_spanned! { self.span =>
            ::spru_script::#registry_trait<#type_parameter_state, #type_parameter_action, #self_type, #field_type>
        };
        registry_bounds.push(bound);
        Ok(())
    }

    fn action_bounds(&self, _context: &Context, _action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        // No action bounds for getters.
        Ok(())
    }

    #[vacro_report::scope]
    fn other_bounds(&self, _context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
        -> syn::Result<()>
    {
        if let FieldKind::Field(field) = &self.field_kind {
            let field_type = &field.ty;
            let bound = syn::parse_quote_spanned! { self.span => 
                #field_type: Clone
            };
            other_bounds.push(bound);
        }
        Ok(())
    }

    #[vacro_report::scope]
    fn registration(&self, context: &Context, stmts: &mut Vec<syn::Stmt>)
        -> syn::Result<()> 
    {
        let Context {
            parameter_registry,
            parameter_registration,
            is_state,
            ..
        } = context;

        let field_ident = self.field_kind.field_ident()?;
        let field_name = self.name_override
            .clone()
            .unwrap_or_else(|| field_ident.to_string());

        let registry_fn_str = if *is_state { "register_state_get" } else { "register_type_get" };
        let registry_fn = syn::Ident::new(registry_fn_str, self.span);

        let wrapper = if self.wrap { quote::quote_spanned! { self.span => ::spru_script::Wrap }} else { TokenStream::new() };

        let stmt = match &self.field_kind {
            FieldKind::Field(_field) => syn::parse_quote_spanned! { self.span =>
                #parameter_registry.#registry_fn(#parameter_registration, #field_name, |this| #wrapper(this.#field_ident.clone()) )?;
            },
            FieldKind::Virtual(impl_item_fn) => {
                let fn_ident = &impl_item_fn.sig.ident;

                syn::parse_quote_spanned! { self.span =>
                    #parameter_registry.#registry_fn(#parameter_registration, #field_name, |this| #wrapper(this.#fn_ident()) )?;
                }
            },
        };

        stmts.push(stmt);
        Ok(())
    }
}

vacro_parser::define! { pub(crate) GetOptions:
    #(options*[,]: GetOption {
        Name: name = #(name: syn::Ident),
        Wrap: wrap = #(wrap: syn::LitBool),
    })
}

impl GetOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let GetOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }

    pub fn wrap(&self) -> bool {
        for option in &self.options {
            if let GetOption::Wrap { wrap } = option {
                return wrap.value;
            }
        }
        false
    }
}
