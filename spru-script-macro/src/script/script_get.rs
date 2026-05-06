use proc_macro2::Span;

use super::{Context, FieldKind};


pub(crate) struct ScriptGet {
    pub span: Span,
    pub name_override: Option<String>,
    pub field_kind: FieldKind,
}

impl ScriptGet {
    pub fn field_type(&self) -> syn::Result<&syn::Type> {
        const ERR_NEEDS_RETURN: &str = "A getter function must return the type of the virtual field";

        match &self.field_kind {
            FieldKind::Field(field) => Ok(&field.ty),
            FieldKind::Virtual(item_fn) => match &item_fn.sig.output {
                syn::ReturnType::Default => Err(syn::Error::new_spanned(&item_fn, ERR_NEEDS_RETURN)),
                syn::ReturnType::Type(_rarrow, ty) => Ok(&**ty),
            },
        }
    }
}

impl super::ScriptImpl for ScriptGet {
    fn self_bounds(&self, context: &Context, self_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
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
            parameter_registry,
            parameter_registration,
            self_type,
        } = context;

        let field_type = self.field_type()?;
        let bound = syn::parse_quote_spanned! { self.span =>
            ::spru_script::RegistryGetter<#type_parameter_state, #type_parameter_action, #self_type, #field_type>
        };
        registry_bounds.push(bound);
        Ok(())
    }

    fn action_bounds(&self, context: &Context, action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        // No action bounds for getters.
        Ok(())
    }

    #[vacro_report::scope]
    fn other_bounds(&self, context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
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
            type_parameter_state,
            type_parameter_action,
            parameter_registry,
            parameter_registration,
            self_type,
        } = context;

        let field_ident = self.field_kind.field_ident()?;
        let field_name = field_ident.to_string();

        let stmt = match &self.field_kind {
            FieldKind::Field(_field) => syn::parse_quote_spanned! { self.span =>
                #parameter_registry.register_get(#parameter_registration, #field_name, |this| this.#field_ident.clone() )?;
            },
            FieldKind::Virtual(impl_item_fn) => {
                let fn_ident = &impl_item_fn.sig.ident;

                syn::parse_quote_spanned! { self.span =>
                    #parameter_registry.register_get(#parameter_registration, #field_name, |this| this.#fn_ident() )?;
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
}
