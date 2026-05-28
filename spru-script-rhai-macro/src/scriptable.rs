use syn::spanned::Spanned as _;

pub(crate) fn scriptable(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let span = input.span();
    let scriptable: spru_script_impl_types::Scriptable = syn::parse2(input)?;

    let details = scriptable.details();
    let spru_script_impl_types::ScriptableDetails {
        self_type,
        generics,
        where_clause,
    } = details;

    let syn::Type::Path(type_path) = &details.self_type else {
        return Err(syn::Error::new_spanned(&details.self_type, "Expected a type path"));
    };
    let macro_ident = type_path.path.segments.last().map(|ps| &ps.ident);

    let action_ty = quote::quote! { Action };

    let registration1 = crate::common::registration_ident();
    let registration2 = quote::quote! { __registration2 };

    let param_this = quote::quote! { $this };
    let param_type_path = quote::quote! { $type_path };

    let vis = quote::quote! { pub };
    let root = quote::quote! { ::spru_script };

    let wrap = quote::quote! { #root::Wrap };
    let phantom = quote::quote! { ::std::marker::PhantomData };

    let prelude = quote::quote_spanned! { span =>
        #[allow(unused_mut)]
        let mut scriptable_type_path = None;
        $(
            scriptable_type_path = Some(spru_script::scriptable_path!(#param_type_path));
        )?
        let #registration2 = #registration1.type_registration(scriptable_type_path);
    };

    let postlude = quote::quote_spanned! {span => 
        #registration2.apply();
    };

    let mut macro_body = proc_macro2::TokenStream::new();
    
    let mut impl_body = proc_macro2::TokenStream::new();



    match &scriptable {
        spru_script_impl_types::Scriptable::State { state } => {
            for member in &state.members {
                let span = member.span();

                let impl_name = mangle_state_member_name(&member);

                // https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html
                macro_body = quote::quote_spanned! { span => 
                    #macro_body
                    (&mut &mut &mut &mut &mut <#param_this>::#impl_name::<#action_ty>()).register_member(&mut #registration2);
                };
                
                let (impl_type, impl_fn, impl_tuple);
                match member {
                    spru_script_impl_types::StateMemberKind::MemberGet { get } => {
                        let name = &get.name;
                        let ty = &get.ty;
                        impl_fn = syn::Ident::new("new_get", span);

                        let impl_arg = match &get.kind {
                            spru_script_impl_types::GetKind::Fn { ident } => 
                                quote::quote_spanned! { span =>
                                    Self::#ident
                                },
                            spru_script_impl_types::GetKind::Field { ident } => 
                                quote::quote_spanned! { span =>
                                    |this| { this.#ident.clone() }
                                },
                        };

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, #impl_arg, #phantom::<#action_ty>)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateGetArgs<#action_ty, Self, #ty>
                        };
                    },
                    spru_script_impl_types::StateMemberKind::MemberSet { set } => {
                        let name = &set.name;
                        let ty = &set.ty;
                        impl_fn = syn::Ident::new("new_set", span);
                        
                        let impl_arg = match &set.kind {
                            spru_script_impl_types::SetKind::Fn { ident } => 
                                quote::quote_spanned! { span =>
                                    Self::#ident
                                },
                            spru_script_impl_types::SetKind::Field { ident } => 
                                quote::quote_spanned! { span =>
                                    |this, value| { this.#ident = value; }
                                },
                        };

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, #impl_arg, #phantom::<#action_ty>)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateSetArgs<#action_ty, Self, #ty>
                        };
                    },
                    spru_script_impl_types::StateMemberKind::MemberCreate { create } => {
                        let name = &create.name;
                        let fn_ident = &create.ident;
                        impl_fn = syn::Ident::new("new_create", span);

                        let action = &create.action;
                        let mut args = params_to_types(&create.params);
                        if !args.empty_or_trailing() {
                            args.push_punct(Default::default());
                        }

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, Self::#fn_ident, #phantom::<#action_ty>)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateCreateArgs<#action_ty, Self, (#args), #action>
                        };
                    },
                    spru_script_impl_types::StateMemberKind::MemberMethod { method } => {
                        let name = &method.name;
                        let fn_ident = &method.ident;
                        impl_fn = syn::Ident::new("new_method", span);
                        let ret_ty = &method.ret;

                        let mut args = params_to_types(&method.params);
                        if !args.empty_or_trailing() {
                            args.push_punct(Default::default());
                        }

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, Self::#fn_ident, #phantom::<#action_ty>)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateMethodArgs<#action_ty, Self, (#args), #ret_ty>
                        };
                    },
                    spru_script_impl_types::StateMemberKind::MemberFunction { function } => {
                        let name = &function.name;
                        let fn_ident = &function.ident;
                        impl_fn = syn::Ident::new("new_function", span);
                        let ret_ty = &function.ret;

                        let mut args = params_to_types(&function.params);
                        if !args.empty_or_trailing() {
                            args.push_punct(Default::default());
                        }

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, Self::#fn_ident, #phantom::<#action_ty>)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateFunctionArgs<#action_ty, (#args), #ret_ty>
                        };
                    },
                };

                impl_body = quote::quote_spanned! { span =>
                    #impl_body
                    #[doc(hidden)]
                    pub fn #impl_name::<#action_ty>() -> #impl_type {
                        #wrap::#impl_fn::<#action_ty>(#impl_tuple)
                    }
                };
            }
        },
        spru_script_impl_types::Scriptable::Ty { ty } => {
            // for member in ty.members {
                
            // }
            // ty.details
            todo!()
        },
    };


    // The where clause is stored separately
    let (impl_generics, _type_generics, _where_clause) = generics.split_for_impl();

    Ok(quote::quote_spanned! { span =>
        macro_rules! __spru_script_rhai {
            ([#param_this:ty] $(#param_type_path:ty)?) => {
                #prelude
                #macro_body
                #postlude
            }
        }
        #vis use __spru_script_rhai as #macro_ident;

        impl #impl_generics #self_type 
        #where_clause 
        {
            #impl_body
        }
    })
}

fn mangle_type_member_name(member: &spru_script_impl_types::TypeMemberKind) -> syn::Ident {
    let (kind, name) = match member {
        spru_script_impl_types::TypeMemberKind::MemberGet { get } => ("get", &get.name),
        spru_script_impl_types::TypeMemberKind::MemberMethod { method } => ("method", &method.name),
        spru_script_impl_types::TypeMemberKind::MemberFunction { function } => ("function", &function.name),
    };

    syn::Ident::new(&format!("__{kind}__{}", name.value()), name.span())
}

fn mangle_state_member_name(member: &spru_script_impl_types::StateMemberKind) -> syn::Ident {
    let (kind, name) = match member {
        spru_script_impl_types::StateMemberKind::MemberGet { get } => ("get", &get.name),
        spru_script_impl_types::StateMemberKind::MemberSet { set } => ("set", &set.name),
        spru_script_impl_types::StateMemberKind::MemberCreate { create } => ("create", &create.name),
        spru_script_impl_types::StateMemberKind::MemberMethod { method } => ("method", &method.name),
        spru_script_impl_types::StateMemberKind::MemberFunction { function } => ("function", &function.name),
    };

    syn::Ident::new(&format!("__{kind}__{}", name.value()), name.span())
}

fn params_to_types(params: &syn::punctuated::Punctuated::<syn::FnArg, syn::Token![,]>) -> syn::punctuated::Punctuated::<syn::Type, syn::Token![,]> {
    params.iter()
        .filter_map(|fn_arg| match fn_arg {
            syn::FnArg::Receiver(_receiver) => None,
            syn::FnArg::Typed(pat_type) => Some(pat_type),
        })
        .map(|pat| {
            &*pat.ty
        })
        .cloned()
        .collect()
}