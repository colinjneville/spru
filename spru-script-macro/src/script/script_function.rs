use proc_macro2::Span;

use super::Context;

pub(crate) struct ScriptFunction {
    pub span: Span,
    pub name_override: Option<String>,
    pub function_fn: syn::ImplItemFn,
}

impl ScriptFunction {
    pub fn name(&self) -> syn::Result<String> {
        let n = self.name_override.clone()
            .unwrap_or_else(|| self.function_fn.sig.ident.to_string());

        Ok(n)
    }

    #[vacro_report::scope]
    pub fn return_type(&self) -> syn::Result<syn::Type> {
        Ok(match &self.function_fn.sig.output {
            syn::ReturnType::Default => syn::parse_quote_spanned!(self.span => ()),
            syn::ReturnType::Type(_, ty) => (**ty).clone(),
        })
    }

    pub fn args(&self) -> syn::Result<impl Iterator<Item = syn::Result<&syn::PatType>>> {
        const ERR_CREATE_RECEIVER: &str = "A `#[create]` function cannot have a receiver (`self`) parameter";

        let iter = self.function_fn.sig.inputs
            .iter()
            .map(|fn_arg| match fn_arg {
                syn::FnArg::Receiver(receiver) => Err(syn::Error::new_spanned(receiver, ERR_CREATE_RECEIVER)),
                syn::FnArg::Typed(pat_type) => Ok(pat_type),
            });

        Ok(iter)
    }

    pub fn arg_pats(&self) -> syn::Result<impl Iterator<Item = syn::Result<&syn::Pat>>> {
        let iter = self.args()?
            .map(|arg| arg.map(|arg| &*arg.pat));
        
        Ok(iter)
    }

    pub fn arg_types(&self) -> syn::Result<impl Iterator<Item = syn::Result<&syn::Type>>> {
        let iter = self.args()?
            .map(|arg| arg.map(|arg| &*arg.ty));

        Ok(iter)
    }

    #[vacro_report::scope]
    pub fn args_pat_type(&self) -> syn::Result<syn::PatType> {
        let arg_pats = self.arg_pats()?.collect::<syn::Result<Vec<_>>>()?;
        let arg_types = self.arg_types()?.collect::<syn::Result<Vec<_>>>()?;

        let pat_type = syn::parse_quote_spanned! { self.span =>
            (#(#arg_pats, )*): (#(#arg_types, )*)
        };

        Ok(pat_type)
    }

    pub fn fn_ident(&self) -> syn::Result<&syn::Ident> {
        Ok(&self.function_fn.sig.ident)
    }
}

impl super::ScriptImpl for ScriptFunction {
    fn self_bounds(&self, _context: &Context, _self_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
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

        let arg_types = self.arg_types()?.collect::<syn::Result<Vec<_>>>()?;
        let return_type = self.return_type()?;

        let registry_trait_str = if *is_state { "RegistryStateFunction" } else { "RegistryTypeFunction" };
        let registry_trait = syn::Ident::new(registry_trait_str, self.span);

        // If arg_types has exactly 1 element, this will (intentionally) be interpreted as a single argument, otherwise
        // it will be an empty tuple, or a tuple of multiple arguments.
        let self_bound = syn::parse_quote_spanned! { self.span => 
            ::spru_script::#registry_trait<#type_parameter_state, #type_parameter_action, #self_type, (#(#arg_types, )*), #return_type>
        };
        registry_bounds.push(self_bound);

        Ok(())
    }

    #[vacro_report::scope]
    fn action_bounds(&self, _context: &Context, _action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        Ok(())
    }

    #[vacro_report::scope]
    fn other_bounds(&self, context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
        -> syn::Result<()>
    {
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
            is_state,
        } = context;

        let name = self.name()?;

        let mut arg_pats = syn::punctuated::Punctuated::<_, syn::Token![,]>::new();
        for arg_pat in self.arg_pats()? {
            arg_pats.push(arg_pat?);
            arg_pats.push_punct(Default::default());
        }

        let fn_ident = self.fn_ident()?;

        let registry_fn_str = if *is_state { "register_state_function" } else { "register_type_function" };
        let registry_fn = syn::Ident::new(registry_fn_str, self.span);

        let stmt = syn::parse_quote_spanned! { self.span => 
            #parameter_registry.#registry_fn(#parameter_registration, #name, |(#arg_pats)| <#self_type>::#fn_ident(#arg_pats) )?;
        };
        
        stmts.push(stmt);

        Ok(())
    }   
}

vacro_parser::define! { pub(crate) FunctionOptions:
    #(options*[,]: FunctionOption {
        Name: name = #(name: syn::Ident),
    })
}

impl FunctionOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let FunctionOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }
}
