use proc_macro2::Span;

use super::Context;

pub(crate) struct ScriptMethod {
    pub span: Span,
    pub name_override: Option<String>,
    pub method_fn: syn::ImplItemFn,
}

impl ScriptMethod {
    pub fn name(&self) -> syn::Result<String> {
        let n = self.name_override.clone()
            .unwrap_or_else(|| self.method_fn.sig.ident.to_string());

        Ok(n)
    }
    
    pub fn raw_return_type(&self) -> syn::Result<&syn::TypeTuple> {
        const ERR_TUPLE_RETURN: &str = "The return type of a method must be a tuple. The first element is the \
         return value in the scripting environment, and any other elements are the Actions produced by the method.";

        let output = &self.method_fn.sig.output;
        match output {
            syn::ReturnType::Type(_, ty) if let syn::Type::Tuple(tty) = &**ty => {
                Ok(tty)
            }
            _ => Err(syn::Error::new_spanned(output, ERR_TUPLE_RETURN)),
        }
    }

    pub fn return_type(&self) -> syn::Result<&syn::Type> {
        const ERR_TUPLE_EMPTY: &str = "The return type of a method must be a tuple with at least one element.";

        let raw = self.raw_return_type()?;
        raw.elems.first()
            .ok_or_else(|| syn::Error::new_spanned(raw, ERR_TUPLE_EMPTY))
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
            (#(#arg_pats),*): (#(#arg_types),*)
        };

        Ok(pat_type)
    }

    pub fn method_ident(&self) -> syn::Result<&syn::Ident> {
        Ok(&self.method_fn.sig.ident)
    }
}

impl super::ScriptImpl for ScriptMethod {
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

        let arg_types = self.arg_types()?;
        let return_type = self.return_type()?;

        // If arg_types has exactly 1 element, this will (intentionally) be interpreted as a single argument, otherwise
        // it will be an empty tuple, or a tuple of multiple arguments.
        let self_bound = syn::parse_quote_spanned! { self.span => 
            ::spru_script::RegistryMethod<#type_parameter_state, #type_parameter_action, #self_type, (#(#arg_types),*), #return_type>
        };
        registry_bounds.push(self_bound);

        Ok(())
    }

    fn action_bounds(&self, context: &Context, action_bounds: &mut syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>)
        -> syn::Result<()> 
    {
        Ok(())
    }

    fn other_bounds(&self, context: &Context, other_bounds: &mut syn::punctuated::Punctuated<syn::WherePredicate, syn::Token![,]>)
        -> syn::Result<()>
    {
        let Context {
            type_parameter_state,
            type_parameter_action,
            parameter_registry,
            parameter_registration,
            self_type,
        } = context;

        let raw_return_type = self.raw_return_type()?;
        let return_type = self.return_type()?;

        let bound = syn::parse_quote_spanned! {self.span => 
            #raw_return_type: spru_script::MethodReturn<#type_parameter_action, T = #return_type>
        };
        other_bounds.push(bound);

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

        let method_ident = self.method_ident()?;
        let name = self.name()?;

        let arg_pats = self.arg_pats()?;
        let arg_pats2 = self.arg_pats()?;

        let stmt = syn::parse_quote_spanned! { self.span => 
            #parameter_registry.register_method(#parameter_registration, #name, |this, (#(#arg_pats),*)| {
                ::spru_script::MethodReturn::convert(this.#method_ident(#(#arg_pats2),*))
            })?;
        };
        
        stmts.push(stmt);
        Ok(())
    }   
}

vacro_parser::define! { pub(crate) MethodOptions:
    #(options*[,]: MethodOption {
        Name: name = #(name: syn::Ident),
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
}
