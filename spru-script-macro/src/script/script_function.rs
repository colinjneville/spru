vacro_parser::define! { pub(crate) FunctionOptions:
    #(options*[,]: FunctionOption {
        Name: name = #(name: syn::Ident),
    })
}

impl FunctionOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            #[allow(irrefutable_let_patterns)]
            if let FunctionOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }
}
