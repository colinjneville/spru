vacro_parser::define! { pub(crate) CreateOptions:
    #(options*[,]: CreateOption {
        Name: name = #(name: syn::Ident),
    })
}

impl CreateOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            #[allow(irrefutable_let_patterns)]
            if let CreateOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }
}
