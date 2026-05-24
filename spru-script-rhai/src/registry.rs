use spru::item::IdT;

pub struct Registry {

}

impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            
        }
    }
}

impl<State, Action> spru_script::Registry<State, Action> for Registry {
    type Registration<'r, Storage> = crate::Registration<'r>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    type RegistrationState<'r, Storage, T: 'static> = crate::RegistrationState<'r>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    type RegistrationType<'r, T: 'static> = crate::RegistrationType<'r>;

    type Error = rhai::EvalAltResult;
}

impl<State, Action, T> spru_script::RegistryState<State, Action, T> for Registry
where
    State: spru::State,
    Action: spru::Action,
    T: spru_script::ScriptableState<State, Action, Self, Type = T>,
    T: spru::item::storage::Storable<State> + 'static,
{
    fn register_state<Storage>(
        &self, 
        registration: &mut Self::Registration<'_, Storage>,
        type_path: Option<spru_script::ScriptablePath>,
    ) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    {
        registration.rhai
            .register_type::<IdT<T>>()
            .register_fn("exists", |ctx: rhai::NativeCallContext<'_>, idt: &mut IdT<T>| {
                let mut handle = crate::LedgerHandle::from_rhai(&ctx);
                let ledger = unsafe { handle.get_mut::<Storage, Action>() };
                ledger.get(*idt).is_ok()
            })
            .register_get("some", |_ctx: rhai::NativeCallContext<'_>, idt: &mut IdT<T>| {
                rhai::Dynamic::from(Some(*idt))
            })
            .register_fn("set_none", |_ctx: rhai::NativeCallContext<'_>, idt: &mut Option<IdT<T>>| {
                *idt = None;
                rhai::Dynamic::UNIT
            })
            .register_get("dynamic", |_ctx: rhai::NativeCallContext<'_>, idt: &mut Option<IdT<T>>| {
                if let Some(idt) = idt {
                    rhai::Dynamic::from(*idt)
                } else {
                    rhai::Dynamic::UNIT
                }
            })
            ;
        
        let mut registration_state = crate::RegistrationState::new(&mut registration.rhai);
        T::register_state::<Storage>(self, &mut registration_state)?;

        let map = registration_state.map;

        if let Some(type_path) = &type_path {
            let mut key = String::new();
            write_scriptable_path(&mut key, type_path);
            registration.static_maps.insert(key.into(), map.into());
        }
        
        Ok(())
    }
}


impl<State, Action, T, U> spru_script::RegistryStateGet<State, Action, T, U> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: spru::item::storage::Storable<State>,
    U: Clone + Sync + Send + 'static,
{
    fn register_state_get<Storage>(&self, registration: &mut crate::RegistrationState, ident: &str, getter: fn(&T) -> U)
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        let f = move |ctx: rhai::NativeCallContext<'_>, this: &mut IdT<T>| -> Result<U, Box<rhai::EvalAltResult>> {
            let mut handle = crate::LedgerHandle::from_rhai(&ctx);
            let ledger = unsafe { handle.get_mut::<Storage, Action>() };
            let item = ledger.get(*this)
                .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(format!("{this:?}: {e}").into(), ctx.call_position())))?;
            Ok(getter(&*item))
        };

        // This roundabout registration is necessary because rhai::Engine::register_get's trait bounds includes
        // a type rhai doesn't publically expose.
        rhai::FuncRegistration::new_getter(ident)
            .with_volatility(true)
            .register_into_engine(registration.rhai, f);

        Ok(())
    }
}

impl<State, Action, T, U> spru_script::RegistryStateSet<State, Action, T, U> for Registry
where 
    State: spru::State,
    Action: spru::Action<State = State>,
    T: spru::item::storage::Storable<State>,
    U: Clone + Sync + Send + 'static,
{
    fn register_state_set<Storage>(&self, registration: &mut crate::RegistrationState, ident: &str, setter: fn(&T, U) -> Vec<Action>)
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        let f = move |ctx: rhai::NativeCallContext<'_>, this: &mut IdT<T>, value: U| -> Result<(), Box<rhai::EvalAltResult>> {
            let mut handle = crate::LedgerHandle::from_rhai(&ctx);
            let mut ledger = unsafe { handle.get_mut::<Storage, Action>() };
            let item = ledger.get(*this)
                .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(format!("{this:?}: {e}").into(), ctx.call_position())))?;
            let actions = setter(&*item, value);

            for action in actions {
                ledger.enqueue_action(this.untyped(), action);
            }
            ledger.flush()
                .map_err(|e| format!("Failed to flush actions: {e}"))?;

            Ok(())
        };

        rhai::FuncRegistration::new_setter(ident)
            .with_purity(false)
            .register_into_engine(registration.rhai, f);

        Ok(())
    }
}


impl<State, Action, T, Args, Ret> spru_script::RegistryStateMethod<State, Action, T, Args, Ret> for Registry
where 
    State: spru::State,
    Action: spru::Action<State = State>,
    T: spru::item::storage::Storable<State>,
    Args: crate::RegisterUnpacked + Clone + Sync + Send + 'static,
    Ret: Clone + Sync + Send + 'static,
{
    fn register_state_method<Storage>(&self, registration: &mut crate::RegistrationState, ident: &str, method: fn(&T, Args) -> (Ret, Vec<Action>))
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        let f = move |ctx: rhai::NativeCallContext<'_>, this: &mut IdT<T>, args: Args| -> Result<Ret, Box<rhai::EvalAltResult>> {
            let mut handle = crate::LedgerHandle::from_rhai(&ctx);
            let mut ledger = unsafe { handle.get_mut::<Storage, Action>() };
            let item = ledger.get(*this)
                .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(format!("{this:?}: {e}").into(), ctx.call_position())))?;
            let (ret, actions) = method(&*item, args);

            for action in actions {
                ledger.enqueue_action(this.untyped(), action);
            }
            ledger.flush()
                .map_err(|e| format!("Failed to flush actions: {e}"))?;

            Ok(ret)
        };

        let reg = rhai::FuncRegistration::new(ident)
            .with_purity(false)
            .with_volatility(true);

        Args::register_unpacked(registration.rhai, reg, f);

        Ok(())
    }
}

impl<State, Action, Create, T: 'static, Args> spru_script::RegistryStateCreate<State, Action, Create, T, Args> for Registry
where 
    State: spru::State,
    Action: spru::Action<State = State>,
    Create: spru::action::Create<T = T> + Into<Action> + 'static,
    T: spru::item::storage::Storable<State>,
    Args: crate::FromArguments + 'static,
{
    fn register_state_create<Storage>(&self, registration: &mut crate::RegistrationState, ident: &str, create: fn(Args) -> Create)
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        #[allow(deprecated)]
        let fn_ptr = rhai::FnPtr::from_fn(ident, move |ctx, mut args| {
            // Self parameter is unused
            args.split_off_first_mut();
            let args = Args::from_arguments(&ctx, args)?;
            let action = create(args);

            let mut handle = crate::LedgerHandle::from_rhai(&ctx);
            let mut ledger = unsafe { handle.get_mut::<Storage, Action>() };
            let id = ledger.enqueue_create(action.into());

            ledger.flush()
                .map_err(|e| format!("Create {}: {e}", std::any::type_name::<T>()))?;

            let idt = id.force_type::<T>();

            Ok(rhai::Dynamic::from(idt))
        }).map_err(|e| *e)?;
        registration.map.insert(ident.into(), fn_ptr.into());

        Ok(())
    }
}

impl<State, Action, T, Args, Ret> spru_script::RegistryStateFunction<State, Action, T, Args, Ret> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: spru::item::storage::Storable<State>,
    Args: crate::FromArguments + 'static,
    Ret: Clone + Send + Sync + 'static,
{
    fn register_state_function<Storage>(&self, registration: &mut crate::RegistrationState, ident: &str, function: fn(Args) -> Ret)
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        #[allow(deprecated)]
        let fn_ptr = rhai::FnPtr::from_fn(ident, move |ctx, mut args| {
            // Self parameter is unused
            args.split_off_first_mut();
            let args = Args::from_arguments(&ctx, args)?;
            Ok(rhai::Dynamic::from(function(args)))
        }).map_err(|e| *e)?;
        registration.map.insert(ident.into(), fn_ptr.into());

        Ok(())
    }
}

impl<State, Action, T> spru_script::RegistryType<State, Action, T> for Registry 
where 
    State: spru::State,
    Action: spru::Action,
    T: spru_script::ScriptableType<State, Action, Registry> + Clone + Sync + Send,
{
    fn register_type<Storage>(
        &self, 
        registration: &mut Self::Registration<'_, Storage>, 
        type_path: Option<spru_script::ScriptablePath>,
    )
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    {
        println!("Registering {}", std::any::type_name::<T>());
        registration.rhai
            .register_type::<T>()
            .register_get("some", |_ctx: rhai::NativeCallContext<'_>, t: &mut T| {
                rhai::Dynamic::from(Some(t.clone()))
            })
            .register_fn("set_none", |_ctx: rhai::NativeCallContext<'_>, t: &mut Option<T>| {
                *t = None;
                rhai::Dynamic::UNIT
            })
            .register_get("dynamic", |_ctx: rhai::NativeCallContext<'_>, t: &mut Option<T>| {
                if let Some(t) = t {
                    rhai::Dynamic::from(t.clone())
                } else {
                    rhai::Dynamic::UNIT
                }
            })
            ;
        
        let mut registration_type = crate::RegistrationType::new(&mut registration.rhai);
        T::register_type(self, &mut registration_type)?;

        let map = registration_type.map;

        if let Some(type_path) = &type_path {
            let mut key = String::new();
            write_scriptable_path(&mut key, type_path);
            registration.static_maps.insert(key.into(), map.into());
        }
        
        Ok(())
    }
}


impl<State, Action, T, U> spru_script::RegistryTypeGet<State, Action, T, U> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: Clone + Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
{
    fn register_type_get(&self, registration: &mut crate::RegistrationType, ident: &str, getter: fn(&T) -> U)
         -> Result<(), Self::Error> 
    {
        let f =  move |this: &mut T| -> U {
            getter(this)
        };
        registration.rhai.register_get(ident, f);

        Ok(())
    }
}

impl<State, Action, T, Args, Ret> spru_script::RegistryTypeMethod<State, Action, T, Args, Ret> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: Clone + Send + Sync + 'static,
    Args: Clone + Send + Sync + 'static,
    Ret: Clone + Send + Sync + 'static,
{
    fn register_type_method(&self, registration: &mut crate::RegistrationType, ident: &str, method: fn(&T, Args) -> Ret)
         -> Result<(), Self::Error> 
    {
        let f = move |this: &mut T, args| -> Ret {
            method(this, args)
        };

        registration.rhai.register_fn(ident, f);
        Ok(())
    }
}

impl<State, Action, T, Args, Ret> spru_script::RegistryTypeFunction<State, Action, T, Args, Ret> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: 'static,
    Args: crate::FromArguments + 'static,
    Ret: Clone + Send + Sync + 'static,
{
    fn register_type_function(&self, registration: &mut crate::RegistrationType, ident: &str, function: fn(Args) -> Ret)
         -> Result<(), Self::Error> 
    {
         #[allow(deprecated)]
        let fn_ptr = rhai::FnPtr::from_fn(ident, move |ctx, mut args| {
            // Self parameter is unused
            args.split_off_first_mut();
            let args = Args::from_arguments(&ctx, args)?;
            Ok(rhai::Dynamic::from(function(args)))
        }).map_err(|e| *e)?;
        registration.map.insert(ident.into(), fn_ptr.into());
        
        Ok(())
    }
}

impl<State, Action, T> spru_script::RegistryTypeEq<State, Action, T> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: Clone + Send + Sync + 'static,
{
    fn register_type_eq(&self, registration: &mut crate::RegistrationType, eq: fn(&T, &T) -> bool)
         -> Result<(), Self::Error> 
    {
        let f_eq = move |a: &mut T, b| {
            eq(a, &b)
        };
        let f_ne = move |a: &mut T, b| {
            !eq(a, &b)
        };

        registration.rhai.register_fn("==", f_eq);
        registration.rhai.register_fn("!=", f_ne);
        Ok(())
    }
}

fn write_scriptable_path(s: &mut String, type_path: &spru_script::ScriptablePath) {
    use std::fmt::Write as _;

    let &spru_script::ScriptablePath(path, type_args) = type_path;

    if let Some((first, rest)) = path.split_first() {
        write!(s, "{first}").unwrap();
        for segment in rest {
            write!(s, "::{segment}").unwrap();
        }

        if let Some((first, rest)) = type_args.split_first() {
            write!(s, "<").unwrap();
            write_scriptable_path(s, first);
            for arg in rest {
                write!(s, ",").unwrap();
                write_scriptable_path(s, arg);
            }
            write!(s, ">").unwrap();
        }
    }
}

// fn assign_to_path(root: &mut rhai::Map, type_path: &spru_script::ScriptablePath, value: rhai::Dynamic) {
//     fn recurse(root: &mut rhai::Map, path: &mut Vec<String>, mut remaining_segments: &[&'static str], mut type_args: &[spru_script::ScriptablePath], key: String, value: rhai::Dynamic) {
//         let next_key = if let Some(&segment) = remaining_segments.split_off_first() {
//             Some(segment.to_string())
//         } else if let Some(type_arg) = type_args.split_off_first() {
//             let mut s = String::new();
//             type_string(&mut s, &type_arg);
            
//             assign_to_path(root, &type_arg, s.as_str().into());
//             Some(s)
//         } else {
//             None
//         };

//         path.push(key);

//         value.as_map_mut()
        
//         let mut map = root.get_mut(&*path[0]).unwrap()
//             .as_map_mut().unwrap();

//         for key in &path[1..] {
//             map = map.get_mut(&**key).unwrap()
//             .as_map_mut().unwrap();
//         }

//         let v = match map.entry(key.into()) {
//             btree_map::Entry::Vacant(ve) => {
//                 if next_key.is_some() {
//                     ve.insert(rhai::Map::new().into())
//                 } else {
//                     ve.insert(value);
//                     return;
//                 }
//             },
//             btree_map::Entry::Occupied(oe) => {
//                 oe.into_mut()
//             },
//         };
        
//         if let Some(next_key) = next_key {
//             recurse(root, path, remaining_segments, type_args, next_key, value);
//         }
//     }
    
//     let &spru_script::ScriptablePath(mut segments, type_args) = type_path;
//     let segments = &mut segments;
//     let segment = segments.split_off_first().expect("ScriptablePath must have at least 1 segment").to_string();
//     recurse(root, &mut vec![], segments, type_args, segment, value);
// }

// // Stringify ScriptablePaths with the format `::path::another_path::Ty<::T,::U<::V,>,>`
// fn type_string(s: &mut String, type_path: &spru_script::ScriptablePath) {
//     use std::fmt::Write as _;

//     for &segment in type_path.0 {
//         write!(s, "::{segment}").unwrap();
//     }
//     if !type_path.1.is_empty() {
//         write!(s, "<").unwrap();
//         for arg in type_path.1 {
//             type_string(s, arg);
//             write!(s, ",").unwrap();
//         }
//         write!(s, ">").unwrap();
//     }
        
// }