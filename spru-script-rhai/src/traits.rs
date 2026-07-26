use std::any;

use spru::item::IdT;
use spru_script::wrap::*;

use crate::MemberRegistration;
use crate::storage_handle::StorageHandle;

// Creates function/method/create traits for converting FromDynamic parameters and IntoDynamic return types
// The first number is the max number of parameters when converting parameters (O(2^n) implementations),
// the second number is the max number of parameters when not converting parameters (O(n))
spru_script_rhai_macro::impl_dynamic_fn!(5, 10);


pub trait RegisterStatelessNoop {
    fn register_stateless(&mut self, _registration: &mut MemberRegistration<'_, '_>) {
        tracing::warn!("Could not register stateless {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterStateless {
    fn register_stateless(&mut self, _registration: &mut MemberRegistration<'_, '_>);
}

pub trait RegisterStateNoop {
    type State: spru::State;
    
    fn register_state<Storage>(&mut self, _registration: &mut MemberRegistration<'_, '_>) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        tracing::warn!("Could not register state {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterState {
    type State: spru::State;
    
    fn register_state<Storage>(&mut self, _registration: &mut MemberRegistration<'_, '_>) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    ;
}

pub trait RegisterStatelessMemberNoop {
    fn register_stateless_member(&mut self, _registration: &mut MemberRegistration<'_, '_>, ) {
        // TODO The type does not give a very good indication of what failed to register,
        // but getting the member name needs some cooperation from the *Wrap types
        tracing::warn!("Could not register member {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterStateMemberNoop {
    type State: spru::State;

    fn register_state_member<Storage>(&mut self, _registration: &mut MemberRegistration<'_, '_>, ) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        // TODO The type does not give a very good indication of what failed to register,
        // but getting the member name needs some cooperation from the *Wrap types
        tracing::warn!("Could not register member {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterStatelessMember<const BITSET: usize> {
    fn register_stateless_member(&mut self, _registration: &mut MemberRegistration<'_, '_>, );
}

pub trait RegisterStateMember<const BITSET: usize> {
    type State: spru::State;

    fn register_state_member<Storage>(&mut self, _registration: &mut MemberRegistration<'_, '_>, ) 
    where
        Storage: spru::item::Storage<State = Self::State>;
}


impl<T> RegisterStatelessNoop for StatelessWrap<T> { }

impl<T> RegisterStateless for &mut StatelessWrap<T>
where 
    T: Clone + Send + Sync + 'static,
{
    fn register_stateless(&mut self, registration: &mut MemberRegistration<'_, '_>) {
        let (_, ) = self.take();
        registration.registration.rhai.register_type::<T>();

        registration.registration.rhai.register_fn("flatten", crate::dynamic::flatten::<T>);
        
        registration.registration.rhai.register_fn("to_array", crate::dynamic::to_array::<T>);

        if let Some((_, statics_map)) = &mut registration.statics_map {
            statics_map.insert("none".into(), rhai::Dynamic::from(None::<T>));

            #[allow(deprecated)]
            let from_array = rhai::FnPtr::from_fn("from_array", crate::dynamic::from_array::<T>)
                .expect("function name must be valid");

            statics_map.insert(
                "from_array".into(), 
                from_array.into(),
            );
        }
    }
}



impl<T> RegisterStatelessMemberNoop for StatelessEqWrap<T> { }

impl<T> RegisterStatelessMember<0> for &mut StatelessEqWrap<T>
where
    T: PartialEq + Clone + Sync + Send + 'static,
{
    fn register_stateless_member(&mut self, registration: &mut MemberRegistration<'_, '_>) {
        let (_, ) = self.take();
        registration.registration.rhai.register_fn("==", move |t: &mut T, t2: T| t == &t2);
        registration.registration.rhai.register_fn("!=", move |t: &mut T, t2: T| t != &t2);
    }
}

impl<T, U> RegisterStatelessMemberNoop for StatelessGetWrap<'_, T, U> { }

impl<T, Args, Ret> RegisterStatelessMemberNoop for StatelessMethodWrap<'_, T, Args, Ret> { }


impl<Args, Ret> RegisterStatelessMemberNoop for StatelessFunctionWrap<'_, Args, Ret> { }

impl<Action, T> RegisterStateNoop for StateWrap<Action, T> 
where
    Action: spru::Action,
{
    type State = Action::State;
}

impl<Action, T> RegisterState for &mut StateWrap<Action, T>
where 
    Action: spru::Action,
    T: spru::item::storage::Storable<Action::State> + Clone + Send + Sync + 'static,
{
    type State = Action::State;

    fn register_state<Storage>(&mut self, registration: &mut MemberRegistration<'_, '_>) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        let (_, ) = self.take();
        registration.registration.rhai.register_type::<IdT<T>>();
        registration.registration.rhai.register_fn("==", |idt: &mut IdT<T>, idt2: IdT<T>| *idt == idt2);
        registration.registration.rhai.register_fn("!=", |idt: &mut IdT<T>, idt2: IdT<T>| *idt != idt2);

        registration.registration.rhai.register_fn("flatten", crate::dynamic::flatten::<IdT<T>>);

        registration.registration.rhai.register_fn("exists", |ctx: rhai::NativeCallContext<'_>, idt: &mut IdT<T>| {
            let mut handle = StorageHandle::from_rhai(&ctx);
            let access = unsafe { handle.get_mut::<Storage, Action>() };
            access.get(*idt).is_ok()
        });
        registration.registration.rhai.register_fn("to_array", crate::dynamic::to_array::<IdT<T>>);

        if let Some((_, statics_map)) = &mut registration.statics_map {
            statics_map.insert("none".into(), rhai::Dynamic::from(None::<IdT<T>>));
            
            #[allow(deprecated)]
            let from_array = rhai::FnPtr::from_fn("from_array", crate::dynamic::from_array::<IdT<T>>)
                .expect("function name must be valid");

            statics_map.insert(
                "from_array".into(), 
                from_array.into(),
            );
        }
    }
}

impl<Action, T, U> RegisterStateMemberNoop for StateGetWrap<'_, Action, T, U>
where
    Action: spru::Action,
{ 
    type State = Action::State;
} 

impl<Action, T, U, Ret> RegisterStateMemberNoop for StateSetWrap<'_, Action, T, U, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

impl<Action, T, Args, Ret> RegisterStateMemberNoop for StateMethodWrap<'_, Action, T, Args, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

impl<Action, Args, Ret> RegisterStateMemberNoop for StateFunctionWrap<'_, Action, Args, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

impl<Action, T, Args, Create> RegisterStateMemberNoop for StateCreateWrap<'_, Action, T, Args, Create>
where
    Action: spru::Action
{ 
    type State = Action::State;
}
