mod script_create;
use script_create::CreateOptions;
mod script_function;
use script_function::FunctionOptions;
mod script_get;
use script_get::GetOptions;
mod script_method;
use script_method::MethodOptions;
mod script_set;
use script_set::SetOptions;

use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned as _;

const ATTR_GET: &str = "get";
const ATTR_SET: &str = "set";
const ATTR_METHOD: &str = "method";
const ATTR_CREATE: &str = "create";
const ATTR_FUNCTION: &str = "function";

const ERR_STATE_ONLY: &str = "The attribute is only supported for State types";

fn require_state(span: Span) -> syn::Result<()> {
    Err(syn::Error::new(span, ERR_STATE_ONLY))
}

enum FieldAttr {
    Get(GetOptions),
    Set(SetOptions),
}

impl FieldAttr {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<(proc_macro2::Span, Self)>> {
        let mut v = vec![];

        for attr in attrs {
            let has_args = !matches!(&attr.meta, syn::Meta::Path(_));
            if attr.meta.path().is_ident(ATTR_GET) {
                let get = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(GetOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Get(get)));
            } else if attr.meta.path().is_ident(ATTR_SET) {
                let set = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(SetOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Set(set)));
            }
        }

        Ok(v)
    }
}

enum FnAttr {
    Get(GetOptions),
    Set(SetOptions),
    Method(MethodOptions),
    Create(CreateOptions),
    Function(FunctionOptions),
}

impl FnAttr {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<(proc_macro2::Span, Self)>> {
        let mut v = vec![];

        for attr in attrs {
            let has_args = !matches!(&attr.meta, syn::Meta::Path(_));
            if attr.meta.path().is_ident(ATTR_GET) {
                let get = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(GetOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Get(get)));
            } else if attr.meta.path().is_ident(ATTR_SET) {
                let set = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(SetOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Set(set)));
            } else if attr.meta.path().is_ident(ATTR_METHOD) {
                let method = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(MethodOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Method(method)));
            } else if attr.meta.path().is_ident(ATTR_CREATE) {
                let create = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(CreateOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Create(create)));
            } else if attr.meta.path().is_ident(ATTR_FUNCTION) {
                let function = has_args.then(|| attr.parse_args())
                    .unwrap_or(Ok(FunctionOptions { options: Default::default() }))?;
                v.push((attr.span(), Self::Function(function)));
            }
        }

        Ok(v)
    }
}

vacro_parser::define! { pub DeriveTraits:
    #(traits*[,]: DeriveTrait {
        Eq: #{Eq},
    })
}

impl Clone for DeriveTrait {
    fn clone(&self) -> Self {
        match self {
            Self::Eq => Self::Eq,
        }
    }
}

vacro_parser::define! { StructOptions:
    #(options*[,]: StructOption {
        Partial: partial = #(partial: syn::Ident),
        Include: include = [#(include*[,]: syn::Ident)],
        State: state = #(is_state: syn::LitBool),
        Derive: derive = [#(derive*[,]: spru_script_base_macro::ScriptableOptionDeriveKind)]
    })
}

impl StructOptions {
    fn partial(&self) -> Option<&syn::Ident> {
        
        for option in &self.options {
            if let StructOption::Partial { partial } = option {
                return Some(partial);
            }
        }
        None
    }

    fn include(&self) -> impl Iterator<Item = &syn::Ident> {
        let mut included = None;
        for option in &self.options {
            if let StructOption::Include { include } = option {
                included = Some(include);
                break;
            }
        }
        
        included.into_iter().flatten()
    }

    fn is_state(&self) -> bool {
        for option in &self.options {
            if let StructOption::State { is_state } = option {
                return is_state.value;
            }
        }

        true
    }

    fn derives(&self) -> impl Iterator<Item = &spru_script_base_macro::ScriptableOptionDeriveKind> {
        let mut iter = None;
        for option in &self.options {
            if let StructOption::Derive { derive } = option {
                iter = Some(derive.iter());
            }
        }

        iter.into_iter().flatten()
    }

    #[vacro_report::scope]
    fn build(self, item_struct: &syn::ItemStruct) -> syn::Result<spru_script_base_macro::Scriptable> {
        let is_state = self.is_state();

        let mut members = if is_state {
            Members::StateMembers(Default::default())
        } else {
            Members::TypeMembers(Default::default())
        };

        for (field_index, field) in item_struct.fields.iter().enumerate() {
            for (span, attr) in FieldAttr::from_attrs(&field.attrs)? {
                let field_ident = field.ident.clone()
                    .unwrap_or_else(|| syn::Ident::new(&field_index.to_string(), Span::call_site()));

                match attr {
                    FieldAttr::Get(get_options) => {
                        let name = get_options.name_override()
                            .unwrap_or(&field_ident);
                        let name = syn::LitStr::new(&name.to_string(), name.span());
                        
                        let member = spru_script_base_macro::Get { 
                            name,
                            ty: field.ty.clone(), 
                            kind: spru_script_base_macro::GetKind::Field { 
                                ident: field_ident,
                            },
                        };

                        match &mut members {
                            Members::StateMembers(state_members) => {
                                state_members.push(member.into());
                            }
                            Members::TypeMembers(type_members) => {
                                type_members.push(member.into());
                            }
                        }
                    },
                    FieldAttr::Set(set_options) => {
                        let name = set_options.name_override()
                            .unwrap_or(&field_ident);
                        let name = syn::LitStr::new(&name.to_string(), name.span());
                        
                        let member = spru_script_base_macro::Set { 
                            name, 
                            ty: field.ty.clone(), 
                            kind: spru_script_base_macro::SetKind::Field {
                                ident: field_ident,
                            }, 
                        };

                        match &mut members {
                            Members::StateMembers(state_members) => {
                                state_members.push(member.into());
                            }
                            Members::TypeMembers(_type_members) => {
                                require_state(span)?;
                            }
                        }
                    },
                };
            }
        }

        let span = item_struct.span();
        let mut generics = item_struct.generics.clone();
        let where_clause = generics.make_where_clause().clone();


        let ident = item_struct.ident.clone();
        let (_, type_generics, _) = generics.split_for_impl();
        let self_type = syn::parse_quote! {
            #ident #type_generics
        };

        let is_state = self.is_state();

        let mut options = spru_script_base_macro::ScriptableOptions::default();

        if self.include().next().is_some() {
            let include = self.include().cloned().collect();
            options.include = Some(spru_script_base_macro::ScriptableOptionInclude { include });
        }

        if let Some(partial) = self.partial().cloned() {
            options.partial = Some(spru_script_base_macro::ScriptableOptionPartial { partial });
        }

        if self.derives().next().is_some() {
            options.derive = Some(spru_script_base_macro::ScriptableOptionDerive { derive: self.derives().cloned().collect() });
        };

        let details = spru_script_base_macro::ScriptableDetails {
            self_type,
            options,
            generics,
            where_clause,
        };

        let ret = match members {
            Members::StateMembers(state_members) => 
                spru_script_base_macro::Scriptable::State { state: spru_script_base_macro::ScriptableState {
                    details,
                    members: state_members,
                }},
            Members::TypeMembers(type_members) =>
                spru_script_base_macro::Scriptable::Ty { ty: spru_script_base_macro::ScriptableType {
                    details,
                    members: type_members,
                }},
        };

        Ok(ret)
    }
}

vacro_parser::define! { ImplOptions:
    #(options*[,]: ImplOption {
        Partial: partial = #(partial: syn::Ident),
        Include: include = [#(include*[,]: syn::Ident)],
        State: state = #(is_state: syn::LitBool),
        Derive: derive = [#(derive*[,]: spru_script_base_macro::ScriptableOptionDeriveKind)]
    })
}

impl ImplOptions {
    fn partial(&self) -> Option<&syn::Ident> {
        for option in &self.options {
            if let ImplOption::Partial { partial } = option {
                return Some(partial);
            }
        }
        None
    }

    fn include(&self) -> impl Iterator<Item = &syn::Ident> {
        let mut included = None;
        for option in &self.options {
            if let ImplOption::Include { include } = option {
                included = Some(include);
                break;
            }
        }
        
        included.into_iter().flatten()
    }

    fn is_state(&self) -> bool {
        for option in &self.options {
            if let ImplOption::State { is_state } = option {
                return is_state.value;
            }
        }
        
        true
    }

    fn derives(&self) -> impl Iterator<Item = &spru_script_base_macro::ScriptableOptionDeriveKind> {
        let mut iter = None;
        for option in &self.options {
            if let ImplOption::Derive { derive } = option {
                iter = Some(derive.iter());
            }
        }

        iter.into_iter().flatten()
    }

    fn build(self, item_impl: &syn::ItemImpl) -> syn::Result<spru_script_base_macro::Scriptable> {
        let is_state = self.is_state();
        let mut members = if is_state { 
            Members::StateMembers(Default::default()) 
        } else { 
            Members::TypeMembers(Default::default())
        };
        
        for sub_item in &item_impl.items {
            if let syn::ImplItem::Fn(impl_item_fn) = sub_item {
                for (span, attr) in FnAttr::from_attrs(&impl_item_fn.attrs)? {
                    match attr {
                        FnAttr::Get(get_options) => {
                            let syn::ReturnType::Type(_, ty) = &impl_item_fn.sig.output else {
                                return Err(syn::Error::new_spanned(&impl_item_fn.sig.output, "A get function must have a return type"));
                            };

                            let fn_ident = impl_item_fn.sig.ident.clone();
                            let name = get_options.name_override()
                                .unwrap_or(&impl_item_fn.sig.ident);
                            let name = syn::LitStr::new(&name.to_string(), name.span());

                            let member = spru_script_base_macro::Get {
                                name,
                                ty: (**ty).clone(),
                                kind: spru_script_base_macro::GetKind::Fn {
                                    ident: fn_ident,
                                },
                            };

                            match &mut members {
                                Members::StateMembers(state_members) => state_members.push(member.into()),
                                Members::TypeMembers(type_members) => type_members.push(member.into()),
                            }
                        },
                        FnAttr::Set(set_options) => {
                            let syn::ReturnType::Type(_, ret) = &impl_item_fn.sig.output else {
                                return Err(syn::Error::new_spanned(&impl_item_fn.sig.output, "A set function must have a return type"));
                            };

                            let arg_err_fn = || syn::Error::new_spanned(&impl_item_fn.sig.inputs, "Expected a single non-receiver parameter");

                            let mut arg_iter = impl_item_fn.sig.inputs.iter()
                                .filter_map(|arg| match arg {
                                    syn::FnArg::Receiver(_receiver) => None,
                                    syn::FnArg::Typed(pat_type) => Some(&*pat_type.ty),
                                });
                            let ty = arg_iter.next()
                                .ok_or_else(arg_err_fn)?;
                            if arg_iter.next().is_some() {
                                return Err(arg_err_fn());
                            }

                            let fn_ident = impl_item_fn.sig.ident.clone();
                            let name = set_options.name_override()
                                .unwrap_or(&impl_item_fn.sig.ident);
                            let name = syn::LitStr::new(&name.to_string(), name.span());

                            let member = spru_script_base_macro::Set {
                                name,
                                ty: ty.clone(),
                                kind: spru_script_base_macro::SetKind::Fn {
                                    ident: fn_ident,
                                    ret: (**ret).clone(),
                                },
                            };

                            match &mut members {
                                Members::StateMembers(state_members) => state_members.push(member.into()),
                                Members::TypeMembers(_type_members) => {
                                    require_state(span)?;
                                }
                            }
                        },
                        FnAttr::Method(method_options) => { 
                            let ident = impl_item_fn.sig.ident.clone();
                            let name = method_options.name_override()
                                .unwrap_or(&ident);
                            let name = syn::LitStr::new(&name.to_string(), name.span());

                            let syn::ReturnType::Type(_, ty) = &impl_item_fn.sig.output else {
                                return Err(syn::Error::new_spanned(&impl_item_fn.sig.output, "A create function must have a return type"));
                            };

                            let (ret, actions) = if let syn::Type::Tuple(ty) = &**ty {
                                let mut iter = ty.elems.clone().into_pairs();

                                let Some(first) = iter.next() else {
                                    return Err(syn::Error::new_spanned(ty, "Expected at least one tuple element, the script return value."));
                                };

                                let ret = first.into_value();
                                let actions = iter.collect();

                                (ret, actions)
                            } else {
                                return Err(syn::Error::new_spanned(ty, "Expected a tuple return value, where the first element is the script return value, and the rest are any mutation Actions."));
                            };

                            let params = impl_item_fn.sig.inputs.clone();

                            let member = spru_script_base_macro::Method {
                                name,
                                ident,
                                ret,
                                actions,
                                params,
                            };                       

                            match &mut members {
                                Members::StateMembers(state_members) => state_members.push(member.into()),
                                Members::TypeMembers(_type_members) => {
                                    require_state(span)?;
                                }
                            }
                        },
                        FnAttr::Create(create_options) => {
                            let ident = impl_item_fn.sig.ident.clone();
                            let name = create_options.name_override()
                                .unwrap_or(&ident);

                            let name = syn::LitStr::new(&name.to_string(), name.span());

                            let syn::ReturnType::Type(_, action) = &impl_item_fn.sig.output else {
                                return Err(syn::Error::new_spanned(&impl_item_fn.sig.output, "A create function must have a return type"));
                            };

                            let params = impl_item_fn.sig.inputs.clone();

                            let member = spru_script_base_macro::Create {
                                name,
                                ident,
                                action: (**action).clone(),
                                params,
                            };

                            match &mut members {
                                Members::StateMembers(state_members) => state_members.push(member.into()),
                                Members::TypeMembers(_type_members) => {
                                    require_state(span)?;
                                },
                            }
                        },
                        FnAttr::Function(function_options) => {
                            let ident = impl_item_fn.sig.ident.clone();
                            let name = function_options.name_override()
                                .unwrap_or(&ident);
                            let name = syn::LitStr::new(&name.to_string(), name.span());

                            let syn::ReturnType::Type(_, ret) = &impl_item_fn.sig.output else {
                                return Err(syn::Error::new_spanned(&impl_item_fn.sig.output, "A function must have a return type"));
                            };

                            let params = impl_item_fn.sig.inputs.clone();

                            let member = spru_script_base_macro::Function {
                                name,
                                ident,
                                ret: (**ret).clone(),
                                params,
                            };

                            match &mut members {
                                Members::StateMembers(state_members) => state_members.push(member.into()),
                                Members::TypeMembers(type_members) => type_members.push(member.into()),
                            }
                        },
                    };
                }
            }
        }

        let ident = (*item_impl.self_ty).clone();

        let span = item_impl.span();
        let mut generics = item_impl.generics.clone();
        let where_clause = generics.make_where_clause().clone();

        let mut options = spru_script_base_macro::ScriptableOptions::default();

        if self.include().next().is_some() {
            let include = self.include().cloned().collect();
            options.include = Some(spru_script_base_macro::ScriptableOptionInclude { include });
        }

        if let Some(partial) = self.partial().cloned() {
            options.partial = Some(spru_script_base_macro::ScriptableOptionPartial { partial });
        }

        if self.derives().next().is_some() {
            options.derive = Some(spru_script_base_macro::ScriptableOptionDerive { derive: self.derives().cloned().collect() });
        };

        let details = spru_script_base_macro::ScriptableDetails { 
            self_type: ident, 
            options,
            generics, 
            where_clause,
        };
        
        let ret = match members {
            Members::StateMembers(state_members) => {
                spru_script_base_macro::Scriptable::State { state: spru_script_base_macro::ScriptableState {
                    details,
                    members: state_members,
                }}
            }
            Members::TypeMembers(type_members) => {
                spru_script_base_macro::Scriptable::Ty { ty: spru_script_base_macro::ScriptableType {
                    details,
                    members: type_members,
                }}
            },
        };

        Ok(ret)
    }
}

#[vacro_report::scope]
pub(crate) fn script_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let item: syn::Item = syn::parse2(item)?;

    let scriptable_macro = match &item {
        syn::Item::Struct(s) => {
            let struct_options: StructOptions = syn::parse2(attr)?;
            Ok(struct_options.build(&s)?)
            
        },
        syn::Item::Impl(i) => {
            let impl_options: ImplOptions = syn::parse2(attr)?;
            Ok(impl_options.build(&i)?)
        },
        _ => Err(syn::Error::new_spanned(&item, "The #[script] attribute can only be applied to structs and impl blocks.")),
    }?;

    use quote::ToTokens as _;
    let err = syn::Error::new(Span::call_site(), scriptable_macro.to_token_stream().to_string()).into_compile_error();

    let item = syn::fold::fold_item(&mut HelperFold, item);

    Ok(quote::quote! {
        #item

        ::spru_script::scriptable!(#scriptable_macro);
        // #err
    })
}

enum Members {
    StateMembers(syn::punctuated::Punctuated<spru_script_base_macro::StateMemberKind, syn::Token![,]>),
    TypeMembers(syn::punctuated::Punctuated<spru_script_base_macro::TypeMemberKind, syn::Token![,]>),
}

struct HelperFold;

impl syn::fold::Fold for HelperFold {
    fn fold_attributes(&mut self, mut i: Vec<syn::Attribute>) -> Vec<syn::Attribute> {
        // Strip out our helper attributes
        // https://github.com/rust-lang/rust/issues/65823
        i.retain(|attr| 
            !attr.path().is_ident(ATTR_GET) 
                && !attr.path().is_ident(ATTR_SET) 
                && !attr.path().is_ident(ATTR_METHOD) 
                && !attr.path().is_ident(ATTR_CREATE) 
                && !attr.path().is_ident(ATTR_FUNCTION) 
        );

        // Attributes can't be nested, so we don't need to continue down the AST
        i
    }
}
