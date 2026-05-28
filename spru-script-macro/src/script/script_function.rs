use proc_macro2::Span;

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
