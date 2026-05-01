struct ScriptablePath(Vec<String>, Vec<Self>);

impl ScriptablePath {
    fn parse(path: &syn::Path) -> Self {
        let path_parts: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let args = &path.segments.last()
            .expect("Path must have at least one segment")
            .arguments;

        let mut arg_paths = vec![];

        if let syn::PathArguments::AngleBracketed(angle_args) = args {
            for arg in &angle_args.args {
                if let syn::GenericArgument::Type(syn::Type::Path(type_path)) = arg {
                    let sub_path = Self::parse(&type_path.path);
                    arg_paths.push(sub_path);
                }
            }
        }
        
        Self(path_parts, arg_paths)
    }
}

impl quote::ToTokens for ScriptablePath {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self(path_parts, arg_paths) = self;

        let expanded = quote::quote! {
            spru_script::ScriptablePath(&[#(#path_parts),*], &[#(#arg_paths),*])
        };

        tokens.extend(expanded);
    }
}

#[proc_macro]
pub fn scriptable_path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path = syn::parse_macro_input!(input as syn::Path);
    let scriptable_path = ScriptablePath::parse(&path);
    quote::quote! {
        #scriptable_path
    }.into()
}

#[test]
fn test_scriptable_path() {
    let path = syn::parse_str::<syn::Path>("MyModule::MyStruct<MyModule::MyOtherStruct<i32, i64>, u32>").unwrap();
    let scriptable_path = ScriptablePath::parse(&path);
    assert_eq!(
        quote::quote! { #scriptable_path }.to_string().chars().filter(|c| !c.is_ascii_whitespace()).collect::<String>(), 
        r#"spru_script::ScriptablePath(
            &["MyModule", "MyStruct"], 
            &[
                spru_script::ScriptablePath(
                    &["MyModule", "MyOtherStruct"], 
                    &[
                        spru_script::ScriptablePath(
                            &["i32"], 
                            &[]
                        ), 
                        spru_script::ScriptablePath(
                            &["i64"], 
                            &[]
                        )
                    ]
                ),
                spru_script::ScriptablePath(
                    &["u32"], 
                    &[]
                )
            ]
        )"#.chars().filter(|c| !c.is_ascii_whitespace()).collect::<String>()
    );
}
