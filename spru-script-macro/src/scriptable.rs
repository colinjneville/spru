use syn::spanned::Spanned as _;

pub(crate) fn scriptable(input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let span = input.span();
    let scriptable: spru_script_base_macro::Scriptable = syn::parse2(input)?;

    let details = scriptable.details();
    let spru_script_base_macro::ScriptableDetails {
        self_type,
        options,
        generics,
        where_clause,
    } = details;

    let syn::Type::Path(type_path) = &details.self_type else {
        return Err(syn::Error::new_spanned(&details.self_type, "Expected a type path"));
    };
    // `MyType` creates macro `register_MyType!`
    let macro_ident = type_path.path.segments.last().map(|ps| syn::Ident::new(&format!("register_{}", ps.ident), ps.ident.span()));

    let action_ty = quote::quote! { Action };

    let registration2 = quote::quote! { __registration2 };

    let param_this = quote::quote! { this };
    let param_type_path = quote::quote! { type_path };

    let vis: syn::Visibility = syn::parse_quote! { pub };
    let root = quote::quote! { ::spru_script::wrap };

    let wrap = quote::quote! { #root::Wrap };
    let phantom = quote::quote! { ::std::marker::PhantomData };

    let type_impl_name = quote::quote! { __type__ };

    

    let postlude = quote::quote_spanned! {span => 
        #registration2.apply();
    };
    
    let mut impl_body = proc_macro2::TokenStream::new();

    let mut impl_names = vec![];

    if let Some(derive) = &options.derive {
        for derive_kind in &derive.derive {
            let derive_fn = syn::Ident::new(&format!("__{}", derive_kind.op_name()), derive.span());
            match derive_kind {
                spru_script_base_macro::ScriptableOptionDeriveKind::Eq => {
                    
                    impl_body = quote::quote_spanned! { span =>
                        #impl_body
                        #[doc(hidden)]
                        pub fn #derive_fn() -> #root::StatelessEqWrap<Self> {
                            #wrap::new_stateless_eq((#phantom, ))
                        }
                    };
                    impl_names.push(derive_fn);
                },
            }
        }
    }

    let is_state;
    match &scriptable {
        spru_script_base_macro::Scriptable::State { state } => {
            is_state = true;
            for member in &state.members {
                let span = member.span();
                let (impl_type, impl_fn, impl_tuple);
                
                match member {
                    spru_script_base_macro::StateMemberKind::MemberGet { get } => {
                        let name = &get.name;
                        let ty = &get.ty;
                        impl_fn = syn::Ident::new("new_state_get", span);

                        let impl_arg = match &get.kind {
                            spru_script_base_macro::GetKind::Fn { ident } => 
                                quote::quote_spanned! { span =>
                                    Self::#ident
                                },
                            spru_script_base_macro::GetKind::Field { ident } => 
                                quote::quote_spanned! { span =>
                                    |this| { this.#ident.clone() }
                                },
                        };

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, #impl_arg, #phantom)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateGetWrap<'static, #action_ty, Self, #ty>
                        };
                    },
                    spru_script_base_macro::StateMemberKind::MemberSet { set } => {
                        let name = &set.name;
                        let ty = &set.ty;
                        impl_fn = syn::Ident::new("new_state_set", span);
                        
                        let field_ret = syn::parse_quote_spanned! { span => 
                            (::spru_util::cloned::Update::<Self>, )
                        };
                        let mut set_ret = &field_ret;
                        let impl_arg = match &set.kind {
                            spru_script_base_macro::SetKind::Fn { ident, ret } => {
                                set_ret = ret;
                                quote::quote_spanned! { span =>
                                    |this, (val, )| this.#ident(val)
                                }
                            }
                            spru_script_base_macro::SetKind::Field { ident } => {
                                quote::quote_spanned! { span =>
                                    |this, (value, )| { 
                                        let mut c = this.clone();
                                        c.#ident = value;
                                        (::spru_util::cloned::update(c), )
                                    }
                                }
                            }
                        };

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, #impl_arg, #phantom)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateSetWrap<'static, #action_ty, Self, (#ty, ), #set_ret>
                        };
                    },
                    spru_script_base_macro::StateMemberKind::MemberCreate { create } => {
                        let name = &create.name;
                        let fn_ident = &create.ident;
                        impl_fn = syn::Ident::new("new_state_create", span);

                        let action = &create.action;
                        let mut args = params_to_types(&create.params);
                        let mut arg_names: syn::punctuated::Punctuated<_, syn::Token![,]> = (0..args.len()).into_iter()
                            .map(|i| syn::Ident::new(&format!("__arg{i}"), span))
                            .collect();

                        if !args.empty_or_trailing() {
                            args.push_punct(Default::default());
                            arg_names.push_punct(Default::default());
                        }

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, |(#arg_names)| { Self::#fn_ident(#arg_names) }, #phantom)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateCreateWrap<'static, #action_ty, Self, (#args), #action>
                        };
                    },
                    spru_script_base_macro::StateMemberKind::MemberMethod { method } => {
                        let name = &method.name;
                        let fn_ident = &method.ident;
                        impl_fn = syn::Ident::new("new_state_method", span);
                        let ret_ty = &method.ret;
                        let actions = &method.actions;

                        let mut args = params_to_types(&method.params);
                        let mut arg_names: syn::punctuated::Punctuated<_, syn::Token![,]> = (0..args.len()).into_iter()
                            .map(|i| syn::Ident::new(&format!("__arg{i}"), span))
                            .collect();

                        if !args.empty_or_trailing() {
                            args.push_punct(Default::default());
                            arg_names.push_punct(Default::default());
                        }

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, |__this, (#arg_names)| { __this.#fn_ident(#arg_names) }, #phantom)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateMethodWrap<'static, #action_ty, Self, (#args), (#ret_ty, #actions)>
                        };
                    },
                    spru_script_base_macro::StateMemberKind::MemberFunction { function } => {
                        let name = &function.name;
                        let fn_ident = &function.ident;
                        impl_fn = syn::Ident::new("new_state_function", span);
                        let ret_ty = &function.ret;

                        let mut args = params_to_types(&function.params);
                        let mut arg_names: syn::punctuated::Punctuated<_, syn::Token![,]> = (0..args.len()).into_iter()
                            .map(|i| syn::Ident::new(&format!("__arg{i}"), span))
                            .collect();

                        if !args.empty_or_trailing() {
                            args.push_punct(Default::default());
                            arg_names.push_punct(Default::default());
                        }

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, |(#arg_names)| { Self::#fn_ident(#arg_names) }, #phantom)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StateFunctionWrap<'static, #action_ty, (#args), #ret_ty>
                        };
                    },
                };

                let impl_name = mangle_state_member_name(&member);

                impl_body = quote::quote_spanned! { span =>
                    #impl_body
                    #[doc(hidden)]
                    pub fn #impl_name<#action_ty>() -> #impl_type {
                        #wrap::#impl_fn(#impl_tuple)
                    }
                };

                impl_names.push(impl_name);
            }

            if options.partial.is_none() {
                impl_body = quote::quote_spanned! { span =>
                    #impl_body
                    #[doc(hidden)]
                    pub fn #type_impl_name<#action_ty>() -> #root::StateWrap<#action_ty, Self> {
                        #wrap::new_state((#phantom, ))
                    }
                };
            }
        },
        spru_script_base_macro::Scriptable::Ty { ty } => {
            is_state = false;
            for member in &ty.members {
                let span = member.span();
                let (impl_type, impl_fn, impl_tuple);
                
                match member {
                    spru_script_base_macro::TypeMemberKind::MemberGet { get } => {
                        let name = &get.name;
                        let ty = &get.ty;
                        impl_fn = syn::Ident::new("new_stateless_get", span);

                        let impl_arg = match &get.kind {
                            spru_script_base_macro::GetKind::Fn { ident } => 
                                quote::quote_spanned! { span =>
                                    Self::#ident
                                },
                            spru_script_base_macro::GetKind::Field { ident } => 
                                quote::quote_spanned! { span =>
                                    |this| { this.#ident.clone() }
                                },
                        };

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, #impl_arg, #phantom)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StatelessGetWrap<'static, Self, #ty>
                        };
                    },
                    spru_script_base_macro::TypeMemberKind::MemberMethod { method } => {
                        let name = &method.name;
                        let fn_ident = &method.ident;
                        impl_fn = syn::Ident::new("new_stateless_method", span);
                        let ret_ty = &method.ret;
                        if method.actions.is_empty() {
                            return Err(syn::Error::new_spanned(&method.actions, "Non-state methods cannot have actions"));
                        }

                        let mut args = params_to_types(&method.params);
                        let mut arg_names: syn::punctuated::Punctuated<_, syn::Token![,]> = (0..args.len()).into_iter()
                            .map(|i| syn::Ident::new(&format!("__arg{i}"), span))
                            .collect();

                        if !args.empty_or_trailing() {
                            args.push_punct(Default::default());
                            arg_names.push_punct(Default::default());
                        }

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, |__this, (#arg_names)| { __this.#fn_ident(#arg_names) }, #phantom)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StatelessMethodWrap<'static, Self, (#args), #ret_ty>
                        };
                    },
                    spru_script_base_macro::TypeMemberKind::MemberFunction { function } => {
                        let name = &function.name;
                        let fn_ident = &function.ident;
                        impl_fn = syn::Ident::new("new_stateless_function", span);
                        let ret_ty = &function.ret;

                        let mut args = params_to_types(&function.params);
                        let mut arg_names: syn::punctuated::Punctuated<_, syn::Token![,]> = (0..args.len()).into_iter()
                            .map(|i| syn::Ident::new(&format!("__arg{i}"), span))
                            .collect();

                        if !args.empty_or_trailing() {
                            args.push_punct(Default::default());
                            arg_names.push_punct(Default::default());
                        }

                        impl_tuple = quote::quote_spanned! { span =>
                            (#name, |(#arg_names)| { Self::#fn_ident(#arg_names) }, #phantom)
                        };

                        impl_type = quote::quote_spanned! { span => 
                            #root::StatelessFunctionWrap<'static, (#args), #ret_ty>
                        };
                    },
                }

                let impl_name = mangle_type_member_name(&member);

                impl_body = quote::quote_spanned! { span =>
                    #impl_body
                    #[doc(hidden)]
                    pub fn #impl_name() -> #impl_type {
                        #wrap::#impl_fn(#impl_tuple)
                    }
                };

                impl_names.push(impl_name);
            }

            if options.partial.is_none() {
                impl_body = quote::quote_spanned! { span =>
                    #impl_body
                    #[doc(hidden)]
                    pub fn #type_impl_name() -> #root::StatelessWrap<Self> {
                        #wrap::new_type((#phantom, ))
                    }
                };
            }
        },
    };
    
    let build_macro_ident = syn::Ident::new("__spru_script__build", span);

    
    // These options are currently (and silently) mutually exclusive. They don't strictly need to be,
    // but it requires a fair amount of work for something unlikely to be actually useful.
    let macros = if let Some(partial) = &options.partial {
        let partial = &partial.partial;

        quote::quote_spanned! { span =>
            macro_rules! __spru_script__partial {
                ($dollar:tt $first_macro:ident $($rest_macro:ident)* $($rest:tt)*) => {
                    $first_macro!($dollar $($rest_macro)* $($rest)* #([#impl_names])*);
                }
            }
            use __spru_script__partial as #partial;
        }
    } else {
        let export_vis = if let syn::Visibility::Public(public) = &vis {
            quote::quote_spanned! { public.span() => 
                #[macro_export]
            }
        } else {
            quote::quote! {}
        };

        let converted_path = span.file()
            .bytes()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        let converted_path = String::from_utf8(converted_path)
            .expect("Vec contains only alphanumeric bytes");

        let unique_macro_str = format!("__{converted_path}_{}_{}_{}_{}", 
            span.start().line, 
            span.start().column,
            span.end().line, 
            span.end().column,
        );
        let unique_macro_ident = syn::Ident::new(&unique_macro_str, span);

        let state_params = is_state.then(|| quote::quote_spanned!(span => <$dollar storage:ty, $dollar action:ty>));
        let state_action_param = is_state.then(|| quote::quote_spanned!(span => ::<$dollar action>));
        let state_storage_param = is_state.then(|| quote::quote_spanned!(span => ::<$dollar storage>));
        let state_trait_fn = syn::Ident::new(if is_state { "register_state" } else { "register_stateless" }, span);
        let state_member_trait_fn = syn::Ident::new(if is_state { "register_state_member" } else { "register_stateless_member" }, span);


        let prelude = quote::quote_spanned! { span =>
            // // TODO If this list grows substantially, it might be worth only including traits
            // // that might actually be used.
            // #[allow(unused_imports)]
            // use spru_script::{
            //     RegisterMember as _,
            //     RegisterTypeNoop as _, RegisterType as _, 
            // };

            #[allow(unused_mut)]
            let mut scriptable_type_path = spru_script::scriptable_path!($dollar #param_this);
            $dollar (
                scriptable_type_path = spru_script::scriptable_path!($dollar #param_type_path);
            )?
            
            let mut #registration2 = $dollar registration1.type_registration(Some(scriptable_type_path));
            (&mut &mut &mut &mut &mut <$dollar #param_this>::#type_impl_name #state_action_param())
                                            .#state_trait_fn #state_storage_param(&mut #registration2);
        };

        // This level of indirection is only necessary when including partials, but using it always simplifies the code
        let build_macro = quote::quote_spanned! { span => 
            macro_rules! #build_macro_ident {
                ($dollar:tt $([$impl_names:ident])*) => {
                    #export_vis
                    macro_rules! #unique_macro_ident {
                        (#state_params $dollar registration1:ident => $dollar #param_this:path $dollar( as $dollar #param_type_path:ty )?) => {
                            {
                                #prelude
                                $(
                                    (&mut &mut &mut &mut &mut <$dollar #param_this>::$impl_names #state_action_param())
                                        .#state_member_trait_fn #state_storage_param(&mut #registration2);
                                )*
                                #postlude
                            }
                        };

                        // TODO this is a temporary workaround to not conflict with telety.
                        // As part of probing for macro existance, telety will invoke
                        // the macro with (0 ...), so if we are called with that, NOOP
                        // so we don't error instead.
                        (0 $dollar ($dollar t:tt)*) => { }
                    }

                    #[allow(non_snake_case)]
                    #vis use #unique_macro_ident as #macro_ident;
                }
            }
        };

        let includes = options.include.as_ref();

        let mut iter = includes
            .iter()
            .flat_map(|include| &include.include)
            .chain(std::iter::once(&build_macro_ident));

        let first_macro = iter.next().expect("Build macro will always be present");

        quote::quote_spanned! { span =>
            #build_macro
            #first_macro!($ #(#iter)* #([#impl_names])*);
        }
    };

    // The where clause is stored separately
    let (impl_generics, _type_generics, _where_clause) = generics.split_for_impl();

    Ok(quote::quote_spanned! { span =>
        #macros

        #[allow(non_snake_case)]
        impl #impl_generics #self_type 
        #where_clause 
        {
            #impl_body
        }
    })
}

fn mangle_type_member_name(member: &spru_script_base_macro::TypeMemberKind) -> syn::Ident {
    let (kind, name) = match member {
        spru_script_base_macro::TypeMemberKind::MemberGet { get } => ("get", &get.name),
        spru_script_base_macro::TypeMemberKind::MemberMethod { method } => ("method", &method.name),
        spru_script_base_macro::TypeMemberKind::MemberFunction { function } => ("function", &function.name),
    };

    syn::Ident::new(&format!("__{kind}__{}", name.value()), name.span())
}

fn mangle_state_member_name(member: &spru_script_base_macro::StateMemberKind) -> syn::Ident {
    let (kind, name) = match member {
        spru_script_base_macro::StateMemberKind::MemberGet { get } => ("get", &get.name),
        spru_script_base_macro::StateMemberKind::MemberSet { set } => ("set", &set.name),
        spru_script_base_macro::StateMemberKind::MemberCreate { create } => ("create", &create.name),
        spru_script_base_macro::StateMemberKind::MemberMethod { method } => ("method", &method.name),
        spru_script_base_macro::StateMemberKind::MemberFunction { function } => ("function", &function.name),
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