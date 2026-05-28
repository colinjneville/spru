use proc_macro2::{Span, TokenStream};

vacro_parser::define! { pub(crate) GetOptions:
    #(options*[,]: GetOption {
        Name: name = #(name: syn::Ident),
        Wrap: wrap = #(wrap: syn::LitBool),
    })
}

impl GetOptions {
    pub fn name_override(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let GetOption::Name { name } = option {
                return Some(name);
            }
        }
        None
    }

    pub fn wrap(&self) -> bool {
        for option in &self.options {
            if let GetOption::Wrap { wrap } = option {
                return wrap.value;
            }
        }
        false
    }
}
