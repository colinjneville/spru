use proc_macro2::Span;

use super::Context;

pub(crate) struct ScriptMethod {
    pub span: Span,
    pub name_override: Option<String>,
    pub method_fn: syn::ImplItemFn,
    pub wrap: bool,
}

impl ScriptMethod {
    const ERR_TUPLE_RETURN: &str = "The return type of a method must be a tuple. The first element is the \
         return value in the scripting environment, and any other elements are the Actions produced by the method.";

    pub fn name(&self) -> syn::Result<String> {
        let n = self.name_override.clone()
            .unwrap_or_else(|| self.method_fn.sig.ident.to_string());

        Ok(n)
    }
    
    pub fn raw_return_type(&self) -> syn::Result<&syn::TypeTuple> {
        let output = &self.method_fn.sig.output;
        match output {
            syn::ReturnType::Type(_, ty) if let syn::Type::Tuple(tty) = &**ty => {
                Ok(tty)
            }
            _ => Err(syn::Error::new_spanned(output, Self::ERR_TUPLE_RETURN)),
        }
    }

    pub fn return_type(&self) -> syn::Result<&syn::Type> {
        const ERR_TUPLE_EMPTY: &str = "The return type of a method must be a tuple with at least one element.";

        let raw = self.raw_return_type()?;
        raw.elems.first()
            .ok_or_else(|| syn::Error::new_spanned(raw, ERR_TUPLE_EMPTY))
        
    }

    pub fn wrapped_return_type(&self) -> syn::Result<syn::Type> {
        let return_type = self.return_type()?;
        let return_type = if self.wrap {
            syn::parse_quote_spanned! { self.span => 
                spru_script::Wrap::<#return_type>
            }
        } else {
            return_type.clone()
        };

        Ok(return_type)
    }

    pub fn action_types(&self) -> syn::Result<impl Iterator<Item = &syn::Type>> {
        let raw = self.raw_return_type()?;
        Ok(raw.elems.iter().skip(1))
    }

    pub fn args(&self) -> syn::Result<impl Iterator<Item = &syn::PatType>> {
        let iter = self.method_fn.sig.inputs
            .iter()
            .filter_map(|fn_arg| match fn_arg {
                syn::FnArg::Receiver(_receiver) => None,
                syn::FnArg::Typed(pat_type) => Some(pat_type),
            });

        Ok(iter)
    }

    pub fn arg_pats(&self) -> syn::Result<impl Iterator<Item = &syn::Pat>> {
        let iter = self.args()?
            .map(|arg| &*arg.pat);
        
        Ok(iter)
    }

    pub fn arg_types(&self) -> syn::Result<impl Iterator<Item = &syn::Type>> {
        let iter = self.args()?
            .map(|arg| &*arg.ty);

        Ok(iter)
    }

    #[vacro_report::scope]
    pub fn args_pat_type(&self) -> syn::Result<syn::PatType> {
        let arg_pats = self.arg_pats()?;
        let arg_types = self.arg_types()?;
        let pat_type = syn::parse_quote_spanned! { self.span =>
            (#(#arg_pats, )*): (#(#arg_types, )*)
        };

        Ok(pat_type)
    }

    pub fn method_ident(&self) -> syn::Result<&syn::Ident> {
        Ok(&self.method_fn.sig.ident)
    }
}

impl super::ScriptImpl for ScriptMethod {
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

        let arg_types = self.arg_types()?;
        let return_type = self.wrapped_return_type()?;

        let registry_trait_str = if *is_state { "RegistryStateMethod" } else { "RegistryTypeMethod" };
        let registry_trait = syn::Ident::new(registry_trait_str, self.span);

        // If arg_types has exactly 1 element, this will (intentionally) be interpreted as a single argument, otherwise
        // it will be an empty tuple, or a tuple of multiple arguments.
        let self_bound = syn::parse_quote_spanned! { self.span => 
            ::spru_script::#registry_trait<#type_parameter_state, #type_parameter_action, #self_type, (#(#arg_types, )*), #return_type>
        };
        registry_bounds.push(self_bound);

        Ok(())
    }

    fn action_bounds(&self, _context: &Context, _action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        Ok(())
    }

    fn other_bounds(&self, context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
        -> syn::Result<()>
    {
        let Context {
            type_parameter_action,
            is_state,
            ..
        } = context;

        if *is_state {
            let raw_return_type = self.raw_return_type()?;
            let return_type = self.return_type()?;

            let bound = syn::parse_quote_spanned! {self.span => 
                #raw_return_type: spru_script::MethodReturn<#type_parameter_action, T = #return_type>
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

        let method_ident = self.method_ident()?;
        let name = self.name()?;

        let arg_pats = self.arg_pats()?;
        let arg_pats2 = self.arg_pats()?;

        let registry_fn_str = if *is_state { "register_state_method" } else { "register_type_method" };
        let registry_fn = syn::Ident::new(registry_fn_str, self.span);

        // TODO this only handles wrapping for methods on States
        let convert_fn = if self.wrap { 
            quote::quote_spanned! { self.span => wrap_convert }
        } else { 
            quote::quote_spanned! { self.span => convert } 
        };

        let fn_body: syn::Expr = if *is_state { 
            syn::parse_quote_spanned! { self.span =>
                ::spru_script::MethodReturn::#convert_fn(this.#method_ident(#(#arg_pats2),*))
            }
        } else {
            syn::parse_quote_spanned! { self.span =>
                this.#method_ident(#(#arg_pats2),*)
            }
        };

        let stmt = syn::parse_quote_spanned! { self.span => 
            #parameter_registry.#registry_fn(#parameter_registration, #name, |this, (#(#arg_pats, )*)| {
                #fn_body
            })?;
        };
        
        stmts.push(stmt);

        Ok(())
    }   
}

vacro_parser::define! { pub(crate) MethodOptions:
    #(options*[,]: MethodOption {
        Name: name = #(name: syn::Ident),
        // TODO probably should just be removed, spru_script::Wrap should just be applied by the 
        // user manually, because generics, tuple returns, etc. won't work cleanly, if at all, 
        // with this macro method
        Wrap: wrap = #(wrap: syn::LitBool),
    })
}

impl MethodOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let MethodOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }

    pub fn wrap(&self) -> bool {
        for option in &self.options {
            if let MethodOption::Wrap { wrap } = option {
                return wrap.value;
            }
        }
        false
    }
}
