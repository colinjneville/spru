vacro_parser::define! { pub(crate) GetOptions:
    #(options*[,]: GetOption {
        Name: name = #(name: syn::Ident),
    })
}

impl GetOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            #[allow(irrefutable_let_patterns)]
            if let GetOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }
}
