vacro_parser::define! { pub(crate) MethodOptions:
    #(options*[,]: MethodOption {
        Name: name = #(name: syn::Ident),
    })
}

impl MethodOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            #[allow(irrefutable_let_patterns)]
            if let MethodOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }
}
