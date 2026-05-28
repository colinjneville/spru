use proc_macro2::Span;

vacro_parser::define! { pub(crate) CreateOptions:
    #(options*[,]: CreateOption {
        Name: name = #(name: syn::Ident),
        Wrap: wrap = #(wrap: syn::LitBool),
    })
}

impl CreateOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let CreateOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }
}
