use proc_macro2::Span;

use super::{Context, FieldKind};

pub(crate) struct ScriptSet {
    pub span: Span,
    pub name_override: Option<String>,
    pub field_kind: FieldKind,
}

impl ScriptSet {
    pub fn field_type(&self) -> syn::Result<&syn::Type> {
        const ERR_MESSAGE: &str = "Setter method must have a single non-receiver parameter with a type equal to the field's type";

        match &self.field_kind {
            FieldKind::Field(field) => {
                Ok(&field.ty)
            }
            FieldKind::Virtual(impl_item_fn) => {
                let mut field_ty = None;
                for arg in &impl_item_fn.sig.inputs {
                    if let syn::FnArg::Typed(pat_type) = arg {
                        let prev = field_ty.replace(&*pat_type.ty);
                        if let Some(_prev) = prev {
                            return Err(syn::Error::new_spanned(pat_type, ERR_MESSAGE));
                        }
                    }
                }        

                field_ty
                    .ok_or_else(|| syn::Error::new_spanned(impl_item_fn, ERR_MESSAGE))
            }
        }
    }

    pub fn name(&self) -> syn::Result<String> {
        match self.name_override.as_ref() {
            Some(field_name_override) => Ok(field_name_override.clone()),
            None => self.field_kind.field_ident()
                .map(ToString::to_string),
        }
    }

    #[vacro_report::scope]
    pub fn action_type(&self, self_type: &syn::Type) -> syn::Result<syn::Type> {
        const ERR_MESSAGE_NO_RETURN: &str = "A setter method must have an Action return type";

        match &self.field_kind {
            FieldKind::Field(_field) => {
                Ok(syn::parse_quote_spanned! { self.span => 
                    ::spru_util::cloned::Update::<#self_type>
                })
            },
            FieldKind::Virtual(impl_item_fn) => {
                let syn::ReturnType::Type(_, action_type) = &impl_item_fn.sig.output else {
                    return Err(syn::Error::new_spanned(&impl_item_fn.sig.output, ERR_MESSAGE_NO_RETURN));
                };

                Ok((**action_type).clone())
            },
        }
    }

    #[vacro_report::scope]
    pub fn action_closure(&self, self_type: &syn::Type) -> syn::Result<syn::ExprClosure> {
        match &self.field_kind {
            FieldKind::Field(_field) => {
                let field_ident = self.field_kind.field_ident()?;
                Ok(syn::parse_quote_spanned! { self.span => 
                    |this, value| { 
                        vec![
                            ::spru_util::cloned::update(
                                #self_type { 
                                    #field_ident: value, 
                                    .. this.clone()
                                }
                            ).into() 
                        ]
                    }
                })
            },
            FieldKind::Virtual(impl_item_fn) => {
                let fn_ident = &impl_item_fn.sig.ident;
                Ok(syn::parse_quote_spanned! { self.span => 
                    |this, value| { ::spru_script::SetReturn::convert(this.#fn_ident(value)) }
                })
            },
        }
    }
}

impl super::ScriptImpl for ScriptSet {
    #[vacro_report::scope]
    fn self_bounds(&self, context: &Context, self_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()>
    {
        if let FieldKind::Field(_field) = &self.field_kind {
            let bound = syn::parse_quote_spanned! { self.span => 
                Clone
            };

            self_bounds.push(bound);
        }

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
            ::spru_script::RegistrySetter<#type_parameter_state, #type_parameter_action, #self_type, #field_type>
        };
        registry_bounds.push(bound);

        Ok(())
    }

    #[vacro_report::scope]
    fn action_bounds(&self, context: &Context, action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        let Context {
            self_type,
            ..
        } = context;

        if let FieldKind::Field(_field) = &self.field_kind {
            let bound = syn::parse_quote_spanned! { self.span => 
                From<::spru_util::cloned::Update::<#self_type>>
            };

            action_bounds.push(bound);
        }
        
        Ok(())
    }

    #[vacro_report::scope]
    fn other_bounds(&self, context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
        -> syn::Result<()>
    {
        let Context {
            type_parameter_action,
            ..
        } = context;

        if let FieldKind::Virtual(field_fn) = &self.field_kind {
            const ERR_TUPLE_RETURN: &str = "The return type of a set method must be a tuple. The elements are the Actions produced by the method.";

            match &field_fn.sig.output {
                syn::ReturnType::Default => return Err(syn::Error::new_spanned(&field_fn.sig, ERR_TUPLE_RETURN)),
                syn::ReturnType::Type(_, return_type) => {
                    let bound = syn::parse_quote_spanned! { self.span => 
                        #return_type: spru_script::SetReturn<#type_parameter_action>
                    };

                    other_bounds.push(bound);
                }
            }
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

        let field_name = self.name()?;

        let action_closure = self.action_closure(self_type)?;

        let stmt = syn::parse_quote_spanned! { self.span =>
            #parameter_registry.register_set(#parameter_registration, #field_name, #action_closure)?;
        };

        stmts.push(stmt);

        Ok(())
    }
}

vacro_parser::define! { pub(crate) SetOptions:
    #(options*[,]: SetOption {
        Name: name = #(name: syn::Ident),
    })
}

impl SetOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let SetOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }
}
