use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned as _;

enum ParamKind {
    // Get: fn(&self) -> ...
    None,
    // Set: fn(&self, Val) -> ...
    One,
    // Method: fn(&self, A, B, C, ...) -> ...
    // Function: fn(A, B, C, ...) -> ...
    // Create: fn(A, B, C, ...) -> ...
    Many,
}

impl ParamKind {
    fn min_params(&self) -> usize {
        match self {
            ParamKind::None => 0,
            ParamKind::One => 1,
            ParamKind::Many => 0,
        }
    }

    fn max_params(&self) -> usize {
        match self {
            ParamKind::None => 0,
            ParamKind::One => 1,
            // More than ever needed, but not enough to overflow
            ParamKind::Many => 1000,
        }
    }
}

enum ReturnKind {
    // Get: fn(&self) -> RetVal
    // Function: fn(...) -> RetVal
    One,
    // Set: fn(&self, Val) -> (Action0, Action1, ...)
    Actions,
    // Method: fn(&self, ...) -> (RetVal, Action0, Action1, ...)
    OnePlusActions,
    // Create: fn(...) -> Action
    Action,
}

impl ReturnKind {
    fn has_return_value(&self) -> bool {
        match self {
            ReturnKind::One => true,
            ReturnKind::Actions => false,
            ReturnKind::OnePlusActions => true,
            ReturnKind::Action => false,
        }
    }
}

struct MemberKind {
    is_state: bool,
    wrapper_ident: syn::Ident,
    param_kind: ParamKind,
    has_receiver: bool,
    return_kind: ReturnKind,
    registration: Option<syn::Expr>,
}

struct TupleArgument {
    ty: syn::Ident,
    unconverted_pat: syn::PatType,
    converted_pat: syn::PatType,
    conversion: syn::Stmt,
    unconverted_bound: syn::WherePredicate,
    converted_bound: syn::WherePredicate,
    tuple_type: syn::Type,
    tuple_value: syn::Type,
    pop_unconverted_arg: syn::Stmt,
    pop_converted_arg: syn::Stmt,
}

impl TupleArgument {
    #[vacro_report::scope]
    fn new(index: usize, span: Span) -> Self {
        let c = (b'A' + index as u8) as char;
        let ty = syn::Ident::new(&format!("{c}"), span);
        let c = (b'a' + index as u8) as char;
        let ident = syn::Ident::new(&format!("{c}"), span);

        let unconverted_pat = syn::parse_quote_spanned!(span => #ident: #ty);
        let converted_pat = syn::parse_quote_spanned!(span => #ident: ::rhai::Dynamic);

        let conversion: syn::Stmt = syn::parse_quote_spanned!(span => let #ident: #ty = FromDynamic::from_dynamic(&ctx, #ident)?; );

        let converted_bound: syn::WherePredicate = syn::parse_quote_spanned!(span => #ty: FromDynamic );
        let unconverted_bound: syn::WherePredicate = syn::parse_quote_spanned!(span => #ty: Clone + Send + Sync + 'static );

        let mut tuple_inner_types = vec![];
        for i in 0..=index {
            let c = (b'A' + i as u8) as char;
            tuple_inner_types.push(syn::Ident::new(&format!("{c}"), span));
        }

        let tuple_type = syn::parse_quote_spanned!(span => (#(#tuple_inner_types, )*));

        let mut tuple_inner_values = vec![];
        for i in 0..=index {
            let c = (b'a' + i as u8) as char;
            tuple_inner_values.push(syn::Ident::new(&format!("{c}"), span));
        }

        let tuple_value = syn::parse_quote_spanned!(span => (#(#tuple_inner_values, )*));

        let pop_unconverted_arg: syn::Stmt = syn::parse_quote_spanned!(span => let #ident: #ty = crate::pop_type(&ctx, &mut args)?; );
        let pop_converted_arg: syn::Stmt = syn::parse_quote_spanned!(span => let #ident: rhai::Dynamic = crate::pop_type(&ctx, &mut args)?; );

        Self {
            ty,
            unconverted_pat,
            converted_pat,
            conversion,
            unconverted_bound,
            converted_bound,
            tuple_type,
            tuple_value,
            pop_unconverted_arg,
            pop_converted_arg,
        }
    }
}

struct TupleArguments { 
    args: Vec<TupleArgument>,
    len: usize,
    empty_tuple: syn::Type,
}

impl TupleArguments {
    fn new(len: usize, span: Span) -> Self {
        let mut args = Vec::with_capacity(len);
        for i in 0..len {
            args.push(TupleArgument::new(i, span));
        }
        let empty_tuple = syn::parse_quote_spanned!(span => ());

        Self {
            args,
            len: 0,
            empty_tuple,
        }
    }

    fn increment(&mut self) {
        self.len += 1;
        assert!(self.len <= self.args.len());
    }

    fn args(&self) -> &[TupleArgument] {
        &self.args[..self.len]
    }

    fn conversions(&self, conversion_bitset: usize) -> impl Iterator<Item = &syn::Stmt> + Clone {
        self.args().iter()
            .enumerate()
            .filter_map(move |(i, ta)| Self::is_converted(conversion_bitset, i).then_some(&ta.conversion))
    }

    fn tuple_type(&self) -> &syn::Type {
        self.args().last()
            .map(|ta| &ta.tuple_type)
            .unwrap_or(&self.empty_tuple)
    }

    // Not actually used as a type, but we pretend for convenience
    fn tuple_value(&self) -> &syn::Type {
        self.args().last()
            .map(|ta| &ta.tuple_value)
            .unwrap_or(&self.empty_tuple)
    }

    fn types(&self) -> impl Iterator<Item = &syn::Ident> + Clone {
        self.args().iter()
            .map(|ta| &ta.ty)
    }

    fn bounds(&self, conversion_bitset: usize) -> impl Iterator<Item = &syn::WherePredicate> + Clone {
        self.args().iter()
            .enumerate()
            .map(move |(i, ta)| {
                if Self::is_converted(conversion_bitset, i) {
                    &ta.converted_bound
                } else {
                    &ta.unconverted_bound
                }
            })
    }

    fn pats(&self, conversion_bitset: usize) -> impl Iterator<Item = &syn::PatType> + Clone {
        self.args().iter()
            .enumerate()
            .map(move |(i, ta)| {
                if Self::is_converted(conversion_bitset, i) {
                    &ta.converted_pat
                } else {
                    &ta.unconverted_pat
                }
            })
    }

    fn pop_args(&self, conversion_bitset: usize) -> impl Iterator<Item = &syn::Stmt> {
        self.args().iter()
            .enumerate()
            .flat_map(move |(i, ta)| {
                if Self::is_converted(conversion_bitset, i) {
                    [Some(&ta.pop_converted_arg), Some(&ta.conversion)]
                } else {
                    [Some(&ta.pop_unconverted_arg), None]
                }
            })
            .flatten()
    }

    fn conversion_count(&self, conversion_bitset: usize) -> usize {
        conversion_bitset.count_ones() as usize
    }

    fn is_converted(conversion_bitset: usize, index: usize) -> bool {
        (conversion_bitset & (1 << index)) != 0
    }
}

struct Ret {
    ident: syn::Ident,
    ty: syn::Ident,
    unconverted_bound: syn::WherePredicate,
    converted_bound: syn::WherePredicate,
    method_unconverted_bound: syn::WherePredicate,
    method_converted_bound: syn::WherePredicate,
    set_bound: syn::WherePredicate,
    create_bound: syn::WherePredicate,
    conversion: syn::Stmt,
    set_to_actions: Vec<syn::Stmt>,
    method_to_ret_actions: syn::Stmt,
    apply_actions: Vec<syn::Stmt>,
    flush: syn::Stmt,
    create: Vec<syn::Stmt>,
}

impl Ret {
    #[vacro_report::scope]
    fn new(span: Span) -> Self {
        let ident = syn::Ident::new("ret", span);
        let ty = syn::Ident::new("Ret", span);

        let unconverted_bound: syn::WherePredicate = syn::parse_quote_spanned!(span => #ty: Clone + Send + Sync + 'static);
        let converted_bound: syn::WherePredicate = syn::parse_quote_spanned!(span => #ty: IntoDynamic);    
        let method_unconverted_bound: syn::WherePredicate = syn::parse_quote_spanned!(span => #ty: spru_script_base::MethodReturn<Action, T: Clone + Send + Sync + 'static> + 'static);
        let method_converted_bound: syn::WherePredicate = syn::parse_quote_spanned!(span => #ty: spru_script_base::MethodReturn<Action, T: IntoDynamic> + 'static);
        let set_bound: syn::WherePredicate = syn::parse_quote_spanned!(span => #ty: spru_script_base::SetReturn<Action> + 'static);
        let create_bound: syn::WherePredicate = syn::parse_quote_spanned!(span => #ty: Into<Action> + 'static);
        
        let conversion: syn::Stmt = syn::parse_quote_spanned!(span => let #ident = IntoDynamic::into_dynamic(#ident););

        let set_to_actions = vec![
            syn::parse_quote_spanned!(span => let actions = #ident.convert(); ),
            syn::parse_quote_spanned!(span => let #ident = (); ),
        ];
        let method_to_ret_actions: syn::Stmt = syn::parse_quote_spanned!(span => let (#ident, actions) = #ident.convert(););

        let apply_actions: syn::Block = syn::parse_quote_spanned! { span => 
            {
                if !actions.is_empty() {
                    for action in actions {
                        ledger.ledger()?.enqueue_action(id, action);
                    }
                    ledger.ledger()?.flush()
                        .map_err(|e| format!("Failed to flush actions: {e}"))?;
                }
            }
        };
        let apply_actions = apply_actions.stmts;

        let create: syn::Block = syn::parse_quote_spanned! { span => 
            {
                let id = ledger.ledger()?.enqueue_create(#ident.into());
                let #ident = rhai::Dynamic::from(id.force_type::<This>());
                ledger.ledger()?.flush()
                    .map_err(|e| format!("Failed to flush actions: {e}"))?;
            }
        };
        let create = create.stmts;

        let flush: syn::Stmt = syn::parse_quote_spanned! { span => 
            if false {
                // ledger.ledger()?.flush()
                //     .map_err(|e| format!("Failed to flush actions: {e}"))?;
            }
        };

        Self {
            ident,
            ty,
            unconverted_bound,
            converted_bound,
            method_unconverted_bound,
            method_converted_bound,
            set_bound,
            create_bound,
            conversion,
            set_to_actions,
            method_to_ret_actions,
            apply_actions,
            create,
            flush,
        }
    }

    fn ty(&self) -> impl Iterator<Item = &syn::Ident> + Clone {
        std::iter::once(&self.ty)
    }

    fn bound(&self, member_kind: &MemberKind, converted: Option<bool>) -> impl Iterator<Item = &syn::WherePredicate> + Clone {
        match &member_kind.return_kind {
            ReturnKind::One => 
                converted.map(|converted| {
                    if converted { &self.converted_bound } else { &self.unconverted_bound }
                }),
            ReturnKind::Actions => Some(&self.set_bound),
            ReturnKind::OnePlusActions => 
                converted.map(|converted| {
                    if converted { &self.method_converted_bound } else { &self.method_unconverted_bound }
                }),
            ReturnKind::Action => Some(&self.create_bound),
        }
        .into_iter()
    }

    fn conversion(&self, converted: Option<bool>) -> impl Iterator<Item = &syn::Stmt> + Clone {
        converted.unwrap_or(false)
            .then_some(&self.conversion)
            .into_iter()
    }

    fn set_to_actions(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        matches!(&member_kind.return_kind, ReturnKind::Actions)
            .then_some(&self.set_to_actions)
            .into_iter()
            .flatten()
    }

    fn method_to_ret_actions(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        matches!(&member_kind.return_kind, ReturnKind::OnePlusActions)
            .then_some(&self.method_to_ret_actions)
            .into_iter()
    }

    fn apply_actions(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        (matches!(&member_kind.return_kind, ReturnKind::Actions | ReturnKind::OnePlusActions))
            .then_some(&self.apply_actions)
            .into_iter()
            .flatten()
    }

    fn create(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        matches!(&member_kind.return_kind, ReturnKind::Action)
            .then_some(&self.create)
            .into_iter()
            .flatten()
    }

    fn flush(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        (!matches!(&member_kind.return_kind, ReturnKind::One))
            .then_some(&self.flush)
            .into_iter()
    }
}

struct RegisterTraits {
    mut_ref: TokenStream,
}

impl RegisterTraits {
    #[vacro_report::scope]
    fn new(span: Span) -> Self {
        let mut_ref = quote::quote_spanned!(span => &mut);

        Self {
            mut_ref,
        }
    }

    fn mut_refs(&self, i: usize) -> impl Iterator<Item = &TokenStream> {
        std::iter::repeat_n(&self.mut_ref, i + 1)
    }
}

struct Receiver {
    param_ident: syn::Ident,
    param_ty: syn::Ident,
    type_bound: syn::WherePredicate,
    state_bound: syn::WherePredicate,
    storable_bound: syn::WherePredicate,
    type_pat: syn::PatType,
    state_pat: syn::PatType,
    // get_storage: Vec<syn::Stmt>,
    get_ledger: Vec<syn::Stmt>,
    lookup: Vec<syn::Stmt>,
    statics_map_get: syn::Stmt,
    statics_map_insert: syn::Stmt,
    format_getter_name: syn::Stmt,
    is_self_check: syn::Stmt,
}

impl Receiver {
    #[vacro_report::scope]
    fn new(span: Span) -> Self {
        let param_ident = syn::Ident::new("this", span);
        let param_ty = syn::Ident::new("This", span);
        let type_ty: syn::Type = syn::parse_quote_spanned!(span => #param_ty);
        let state_ty: syn::Type = syn::parse_quote_spanned!(span => IdT<#param_ty>);
        let type_bound = syn::parse_quote_spanned!(span => #type_ty: Clone + Send + Sync + 'static);
        let state_bound = syn::parse_quote_spanned!(span => #state_ty: Clone + Send + Sync + 'static);
        let storable_bound = syn::parse_quote_spanned!(span => #param_ty: spru::item::storage::Storable<Action::State>);
        let type_pat = syn::parse_quote_spanned!(span => #param_ident: &mut #type_ty);
        let state_pat = syn::parse_quote_spanned!(span => #param_ident: &mut #state_ty);

        // Vec<Stmt> is not directly parseable
        let get_ledger_block: syn::Block = syn::parse_quote_spanned! { span => 
            {
                let mut handle = crate::LedgerHandle::from_rhai(&ctx);
                let mut ledger = unsafe { handle.get_mut::<Storage, Action>() };
            }
        };
        let get_ledger = get_ledger_block.stmts;

        let lookup_block: syn::Block = syn::parse_quote_spanned! { span => 
            {
                let id = #param_ident.untyped();
                let item = ledger.get(*#param_ident)
                    .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(format!("{this:?}: {e}").into(), ctx.call_position())))?;
                let #param_ident = &*item;
            }
        };
        let lookup = lookup_block.stmts;

        let statics_map_get: syn::Stmt = syn::parse_quote_spanned! { span => 
            let Some((_, statics_map)) = registration.statics_map.as_mut() else { return; };
        };

        let statics_map_insert: syn::Stmt = syn::parse_quote_spanned! { span => 
            statics_map.insert(name.into(), fn_ptr.into());
        };

        let format_getter_name: syn::Stmt = syn::parse_quote_spanned! { span => 
            let getter_name = format!("get${name}");
        };

        let is_self_check: syn::Stmt = syn::parse_quote_spanned! { span =>
            if let Ok(current) = ctx.call_fn_raw(&getter_name, true, true, &mut [&mut rhai::Dynamic::from(#param_ident.clone())]) 
                && ctx.call_fn("==", (a.clone(), current)).ok() == Some(true) 
            { 
                return Ok(());
            }
        };
        
        Self {
            param_ident,
            param_ty,
            type_bound,
            state_bound,
            storable_bound,
            type_pat,
            state_pat,
            // get_storage,
            get_ledger,
            lookup,
            statics_map_get,
            statics_map_insert,
            format_getter_name,
            is_self_check,
        }
    }

    fn ident(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Ident> + Clone {
        member_kind.has_receiver.then_some(&self.param_ident)
            .into_iter()
    }

    fn param_ty(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Ident> + Clone {
        // Create stills needs this parameter to convert the Id to IdT<T>
        (matches!(&member_kind.return_kind, ReturnKind::Action) || member_kind.has_receiver).then_some(&self.param_ty)
            .into_iter()
    }

    fn bound(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::WherePredicate> + Clone {
        member_kind.has_receiver.then_some(if member_kind.is_state { &self.state_bound } else { &self.type_bound })
            .into_iter()
    }

    fn storable_bound(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::WherePredicate> + Clone {
        ((matches!(&member_kind.return_kind, ReturnKind::Action) || member_kind.has_receiver) && member_kind.is_state)
            .then_some(&self.storable_bound)
            .into_iter()
    }

    fn pat(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::PatType> + Clone {
        member_kind.has_receiver.then_some(if member_kind.is_state { &self.state_pat } else { &self.type_pat })
            .into_iter()
    }

    // fn get_storage(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
    //     ((member_kind.has_receiver || matches!(&member_kind.return_kind, ReturnKind::Action)) && member_kind.is_state)
    //         .then_some(&self.get_storage)
    //         .into_iter()
    //         .flatten()
    // }

    fn get_ledger(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        ((member_kind.has_receiver || matches!(&member_kind.return_kind, ReturnKind::Action)) && member_kind.is_state)
            .then_some(&self.get_ledger)
            .into_iter()
            .flatten()
    }

    fn lookup(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        (member_kind.has_receiver && member_kind.is_state)
            .then_some(&self.lookup)
            .into_iter()
            .flatten()
    }

    fn statics_map_get(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        (!member_kind.has_receiver)
            .then_some(&self.statics_map_get)
            .into_iter()
    }

    fn statics_map_insert(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        (!member_kind.has_receiver)
            .then_some(&self.statics_map_insert)
            .into_iter()
    }

    fn format_getter_name(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        (member_kind.is_state && matches!(&member_kind.return_kind, ReturnKind::Actions))
            .then_some(&self.format_getter_name)
            .into_iter()
    }

    fn is_self_check(&self, member_kind: &MemberKind) -> impl Iterator<Item = &syn::Stmt> + Clone {
        (member_kind.is_state && matches!(&member_kind.return_kind, ReturnKind::Actions))
            .then_some(&self.is_self_check)
            .into_iter()
    }
}

#[vacro_report::scope]
fn make_register_fn(member_kind: &MemberKind, receiver: &Receiver, tuple_args: &TupleArguments, ret: &Ret, conversion_bitset: usize, return_conversion: Option<bool>, span: Span) -> syn::Block {
    let receiver_get_ledger = receiver.get_ledger(member_kind);

    let args_tuple_value = if member_kind.param_kind.max_params() != 0 { Some(tuple_args.tuple_value()) } else { None };
    let args_tuple_value = args_tuple_value.iter();

    let ret_conversion = ret.conversion(return_conversion);
    let ret_flush = ret.flush(member_kind);

    if let Some(registration) = &member_kind.registration {
        let receiver_pat = receiver.pat(member_kind);
        let receiver_ident = receiver.ident(member_kind);
        let receiver_lookup = receiver.lookup(member_kind);
        let receiver_format_getter_name = receiver.format_getter_name(member_kind);
        let receiver_is_self_check = receiver.is_self_check(member_kind);

        let args_pats = tuple_args.pats(conversion_bitset);
        let args_conversions = tuple_args.conversions(conversion_bitset);
 
        let ret_set_to_actions = ret.set_to_actions(member_kind);
        let ret_method_to_ret_actions = ret.method_to_ret_actions(member_kind);
        let ret_apply_actions = ret.apply_actions(member_kind);

        syn::parse_quote_spanned! { span => 
            {
                #(#receiver_format_getter_name)*
                let reg = #registration;
                let closure = 
                    move |
                        ctx: rhai::NativeCallContext<'_>, 
                        #(#receiver_pat, )* 
                        #(#args_pats, )*
                    |
                        -> Result<_, Box<rhai::EvalAltResult>>  
                    {
                        #(#receiver_is_self_check)*

                        #(#receiver_get_ledger)*
                        #(#receiver_lookup)*
                        #(#args_conversions)*
                        let ret = method(#(#receiver_ident, )* #(#args_tuple_value, )* );
                        #(#ret_set_to_actions)*
                        #(#ret_method_to_ret_actions)*
                        #(#ret_conversion)*
                        #(#ret_apply_actions)*
                        #(#ret_flush)*
                        
                        Ok(ret)
                    };
                reg.register_into_engine(&mut registration.registration.rhai, closure);
            }
        }
    } else {
        let receiver_statics_map_get = receiver.statics_map_get(member_kind);
        let receiver_statics_map_insert = receiver.statics_map_insert(member_kind);

        let args_pop_args = tuple_args.pop_args(conversion_bitset);
        
        let ret_create = ret.create(member_kind);
        let ret_ident = &ret.ident;

        syn::parse_quote_spanned! { span => 
            {
                #(#receiver_statics_map_get)*

                #[allow(deprecated)]
                let Ok(fn_ptr) = rhai::FnPtr::from_fn(name, move |ctx, mut args| {
                    // Self parameter is unused
                    args.split_off_first_mut();
                    #(#args_pop_args)*
                    extra_args_error(&ctx, args)?;

                    let ret = method(#(#args_tuple_value, )* );

                    // Create-only
                    #(#receiver_get_ledger)*
                    #(#ret_create)*
                    #(#ret_flush)*
                    
                    #(#ret_conversion)*
                    Ok(rhai::Dynamic::from(#ret_ident))
                }) else {
                    // TODO delegate to the blanket implementation if we encounter a reserved name
                    unimplemented!()
                    // <StateCreateWrap<'_, Action, T, Args, Create> as RegisterMemberNoop>::register_member::<Storage>(self, registration);
                };

                #(#receiver_statics_map_insert)*
            }
        }
    }
}

vacro_parser::define!{Input:
    #(converted_params_limit: syn::LitInt), #(unconverted_params_limit: syn::LitInt)
}

#[vacro_report::scope]
pub(crate) fn impl_dynamic_fn(input: TokenStream) -> syn::Result<TokenStream> {
    let span = input.span();
    let input: Input = syn::parse2(input)?;
    let converted_params_limit: usize = input.converted_params_limit.base10_parse()
        .expect("Expected an unsigned integer limit");
    let unconverted_params_limit: usize = input.unconverted_params_limit.base10_parse()
        .expect("Expected an unsigned integer limit");
    assert!(converted_params_limit <= unconverted_params_limit);

    let mut output = TokenStream::new();

    let member_kinds = vec![
        MemberKind {
            is_state: true,
            wrapper_ident: syn::Ident::new("StateGetWrap", span),
            param_kind: ParamKind::None,
            has_receiver: true,
            return_kind: ReturnKind::One,
            registration: Some(syn::parse_quote_spanned!(span => rhai::FuncRegistration::new_getter(name).with_volatility(true))),
        },
        MemberKind {
            is_state: true,
            wrapper_ident: syn::Ident::new("StateSetWrap", span),
            param_kind: ParamKind::One,
            has_receiver: true,
            return_kind: ReturnKind::Actions,
            registration: Some(syn::parse_quote_spanned!(span => rhai::FuncRegistration::new_setter(name).with_purity(true))),
        },
        MemberKind {
            is_state: true,
            wrapper_ident: syn::Ident::new("StateMethodWrap", span),
            param_kind: ParamKind::Many,
            has_receiver: true,
            return_kind: ReturnKind::OnePlusActions,
            registration: Some(syn::parse_quote_spanned!(span => rhai::FuncRegistration::new(name).with_purity(true).with_volatility(true))),
        },
        MemberKind {
            is_state: true,
            wrapper_ident: syn::Ident::new("StateFunctionWrap", span),
            param_kind: ParamKind::Many,
            has_receiver: false,
            return_kind: ReturnKind::One,
            registration: None,
        },
        MemberKind {
            is_state: true,
            wrapper_ident: syn::Ident::new("StateCreateWrap", span),
            param_kind: ParamKind::Many,
            has_receiver: false,
            return_kind: ReturnKind::Action,
            registration: None,
        },
        MemberKind {
            is_state: false,
            wrapper_ident: syn::Ident::new("StatelessGetWrap", span),
            param_kind: ParamKind::None,
            has_receiver: true,
            return_kind: ReturnKind::One,
            registration: Some(syn::parse_quote_spanned!(span => rhai::FuncRegistration::new_getter(name).with_volatility(false))),
        },
        MemberKind {
            is_state: false,
            wrapper_ident: syn::Ident::new("StatelessMethodWrap", span),
            param_kind: ParamKind::Many,
            has_receiver: true,
            return_kind: ReturnKind::One,
            // Non-pure non-state methods may be derirable at some point, but need a way to prevent mutating the game state
            registration: Some(syn::parse_quote_spanned!(span => rhai::FuncRegistration::new(name).with_purity(true).with_volatility(false))),
        },
        MemberKind {
            is_state: false,
            wrapper_ident: syn::Ident::new("StatelessFunctionWrap", span),
            param_kind: ParamKind::Many,
            has_receiver: false,
            return_kind: ReturnKind::One,
            registration: None,
        },
    ];

    let no_return_def = [None];
    let yes_return_def = [Some(false), Some(true)];
    
    let register_traits = RegisterTraits::new(span);
    let mut tuple_args = TupleArguments::new(unconverted_params_limit, span);
    let ret = Ret::new(span);
    let receiver = Receiver::new(span);

    let stateless_fn = syn::Ident::new("register_stateless_member", span);
    let state_fn = syn::Ident::new("register_state_member", span);

    let stateless_trait = syn::Ident::new("RegisterStatelessMember", span);
    let state_trait = syn::Ident::new("RegisterStateMember", span);

    for param_count in 0..unconverted_params_limit {
        for member_kind in &member_kinds {
            let MemberKind { 
                is_state,
                wrapper_ident,
                param_kind,
                return_kind,
                ..
            } = member_kind;

            if param_count < param_kind.min_params() || param_count > param_kind.max_params() {
                continue;
            }

            let return_conversions = if return_kind.has_return_value() { yes_return_def.as_slice() } else { no_return_def.as_slice() };

            let action_param = is_state.then(|| quote::quote_spanned!(span => Action,));
            let action_param = action_param.as_ref();
            let action_bound = is_state.then(|| quote::quote_spanned!(span => Action: ::spru::Action,));
            let action_bound = action_bound.as_ref();
            let state_assoc: Option<syn::ItemType> = is_state.then(|| syn::parse_quote_spanned!(span => type State = Action::State;));
            let state_assoc = state_assoc.as_ref();
            let storage_param = is_state.then(|| syn::Ident::new("Storage", span));
            let storage_param = storage_param.as_ref();
            let storage_bound = storage_param.map(|storage_param| quote::quote_spanned!(span => #storage_param: spru::item::Storage<State = Self::State>,));
            let storage_bound = storage_bound.as_ref();
            let trait_fn = if *is_state { &state_fn } else { &stateless_fn };
            let trait_ident = if *is_state { &state_trait } else { &stateless_trait };

            for &return_conversion in return_conversions {
                let bitset_max = if param_count < converted_params_limit { 1usize << param_count } else { 1 };
                for conversion_bitset in 0 .. bitset_max {
                    let args_converted_count = tuple_args.conversion_count(conversion_bitset);
                    let total_converted_count = args_converted_count + return_conversion.unwrap_or(false) as usize;

                    let receiver_param_ty = receiver.param_ty(member_kind);
                    let receiver_param_ty2 = receiver.param_ty(member_kind);
                    let receiver_bound = receiver.bound(member_kind);
                    let receiver_storable_bound = receiver.storable_bound(member_kind);

                    let args_bounds = tuple_args.bounds(conversion_bitset);
                    let args_types = tuple_args.types();
                    let args_tuple_type = if member_kind.param_kind.max_params() != 0 { Some(tuple_args.tuple_type()) } else { None };
                    let args_tuple_type = args_tuple_type.iter();

                    let ret_ty = ret.ty();
                    let ret_ty2 = ret.ty();
                    let ret_bound = ret.bound(member_kind, return_conversion);
                    
                    let mut_refs = register_traits.mut_refs(total_converted_count);

                    let mut trait_bitset = conversion_bitset;
                    if let Some(return_conversion) = return_conversion {
                        trait_bitset <<= 1;
                        trait_bitset |= return_conversion as usize;
                    }

                    let register_fn = make_register_fn(member_kind, &receiver, &tuple_args, &ret, conversion_bitset, return_conversion, span);

                    output = quote::quote_spanned! { span =>
                        #output
                        #[allow(warnings)]
                        impl<#action_param #(#receiver_param_ty, )* #(#args_types, )* #(#ret_ty, )*> 
                            #trait_ident <#trait_bitset> for 
                            #(#mut_refs)* #wrapper_ident<
                                '_, 
                                #action_param
                                #(#receiver_param_ty2, )*
                                #(#args_tuple_type, )*
                                #(#ret_ty2, )*
                            >
                        where
                            #action_bound
                            #(#receiver_bound, )*
                            #(#receiver_storable_bound, )*
                            #(#args_bounds, )*
                            #(#ret_bound, )*
                        {
                            #state_assoc

                            fn #trait_fn <#storage_param> (&mut self, registration: &mut Registration2<'_, '_>, ) 
                            where
                                #storage_bound
                            {
                                let (name, method, _phantom) = self.take();
                                #register_fn
                            }
                        }
                    };
                }
            }
        }

        tuple_args.increment();
    }

    Ok(output)
}