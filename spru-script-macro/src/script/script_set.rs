vacro_parser::define! { pub(crate) SetOptions:
    #(options*[,]: SetOption {
        Name: name = #(name: syn::Ident),
    })
}

impl SetOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            #[allow(irrefutable_let_patterns)]
            if let SetOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }
}
