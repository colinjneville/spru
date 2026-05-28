use proc_macro2::Span;

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
