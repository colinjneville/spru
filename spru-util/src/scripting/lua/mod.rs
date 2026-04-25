//! THe lua implementation works as follows:
//! 
//! The mlua instance requires all of its contents be 'static, but converting
//! [IdT<T>]s to `T` requires a [spru::interactor::Ledger] with a borrow to the [spru::item::Storage].
//! mlua has a feature to add scoped user data with non-static lifetimes, but with the catch that
//! this data can't be converted back once it is type erased (as it is not [std::any::Any]).
//! That means our lookups for each possible `T` must be a method of our ledger user data.
//! 
//! When visiting each sub-State type, we create a closure for each `T` of type [MappingFn], and key them
//! by the [IdT<T>] type id. Each scriptable field/method creates a sub-closure of type [GetterFn], etc 
//! to utilize the `T`.
//! 
//! The game's root object is inserted as a global at [ROOT_KEY] ("root").
//! 
//! When a lua script is run, a scope is created with [LuaLedger] temporarily created for the duration.
//! The ledger is inserted as a global at [LEDGER_KEY]. When a field or method on `T` is accessed, it accesses
//! the [LuaLedger], then calls the get/set/method method on it. The arguments to the method include the [IdT]
//! and the [GetterFn] to do the desired access. The method then maps the [IdT] type id to get the correct [MappingFn],
//! which gets the `T` from the ledger and passes it to the [GetterFn]. This returns any lua Value.

pub(crate) mod func;
mod game_init;
pub use game_init::GameInit;
mod instance;
pub use instance::Lua;
mod interaction;
pub use interaction::Interaction;
pub(crate) mod key;
mod ledger;
use ledger::Ledger;
mod registration;
use registration::{Registration, RegistrationType};
pub mod registry;
use registry::Registry;
pub mod script;
pub use script::Script;

use spru::item::IdT;

pub trait IntoLua {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value>;
}

// Perhaps one day...
// https://github.com/rust-lang/rfcs/issues/2758
macro_rules! forward_into_lua {
    ($(<>)? $($t:ty),+ $(,)?) => {
        $(
            impl IntoLua for $t {
                fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
                    mlua::IntoLua::into_lua(self, lua)
                }
            }
        )+
    }
}

// https://docs.rs/mlua/latest/mlua/trait.IntoLua.html#foreign-impls
forward_into_lua! {
    &str,
    &std::ffi::CStr,
    &std::ffi::OsStr,
    &std::path::Path,
    // &BStr,
    std::borrow::Cow<'_, str>,
    std::borrow::Cow<'_, std::ffi::CStr>,
    bool,
    char,
    f32,
    f64,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    Box<str>,
    std::ffi::CString,
    String,
    std::ffi::OsString,
    std::path::PathBuf,
}

// TODO generic impls

impl<T: 'static> IntoLua for IdT<T> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        mlua::IntoLua::into_lua(lua.create_any_userdata(self)?, lua)
    }
}

#[cfg(test)]

mod test {
    use tagset::tagset;

    use spru::item::IdT;

    use crate::scripting::{self, lua};

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct X {
        a: i32,
        // b: IdT<X>,
    }

    impl<State, Action, Registry> scripting::ScriptableType<State, Action, Registry> for X
    where 
        State: spru::State,
        Action: spru::Action,
        Registry: 
            scripting::RegistryGetter<State, Action, Self, i32>,
    {
        fn register<Storage>(registry: &Registry, registration: &mut Registry::MemberRegistration<'_, Storage, Self>)
             -> Result<(), Registry::Error> 
        where
            Storage: spru::item::Storage<State = State>,
        {
            registry.register_get(registration, "a", |this| this.a)?;
            Ok(())
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct Y {
        b: IdT<X>,
    }

    impl<State, Action, Registry> scripting::ScriptableType<State, Action, Registry> for Y
    where 
        State: spru::State,
        Action: spru::Action,
        Registry: 
            scripting::RegistryGetter<State, Action, Self, IdT<X>>,
    {
        fn register<Storage>(registry: &Registry, registration: &mut Registry::MemberRegistration<'_, Storage, Self>) 
            -> Result<(), Registry::Error> 
        where
            Storage: spru::item::Storage<State = State>,
        {
            registry.register_get(registration, "b", |this| this.b)?;
            Ok(())
        }
    }

    #[tagset(impl tagset::proxy::serde::Serialize)]
    #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
    #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
    #[tagset(impl spru::State)]
    #[tagset(impl<Action, Registry> scripting::ScriptableState<Action, Registry>)]
    #[tagset(derive(Debug))]
    #[tagset(X)]
    #[tagset(Y)]
    struct MyState;

    #[tagset(impl spru::Action {
        type State = MyState;
    })]
    #[tagset(impl tagset::proxy::serde::Serialize)]
    #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
    #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
    #[tagset(derive(Debug, Clone))]
    #[tagset(include(crate::cloned::Actions<X>))]
    #[tagset(include(crate::cloned::Actions<Y>))]
    struct MyAction;


    #[test]
    fn register_type() {
        let storage = crate::storage::Standalone::<MyState>::new();

        let lua = lua::Lua::new();

        // let game_init = game_init::GameInit::new(lua.script("
        //     root.b.a
        // "));

        let mut test_interactor = spru::interactor::test_util::TestInteractor::new(storage);

        let mut interactor = test_interactor.interactor::<MyAction, _>(());
        let x = interactor
            .create(crate::cloned::create(X { a: 3 }));
        let y = interactor
            .create(crate::cloned::create(Y { b: x.id() }));
        let root = y.id();

        interactor.flush().unwrap();

        
        // let interaction = interaction::Interaction::<MyState, MyAction, IdT<Y>, i32>
        //     ::new(lua, "root.b.a".to_string());

        // spru::Server::init(game_init, player_init, reaction)


        // spru::Interaction::apply(&interaction, &mut interactor)
        //     .unwrap();

        // let mut interactor = test_interactor.interactor::<MyAction, _>(());
        // let x = interactor
        //     .create(crate::cloned::create(X { a: 3 }));
        // let y = interactor
        //     .create(crate::cloned::create(Y { b: x.id() }));
        // let root = y.id();

        // interactor.flush().unwrap();
        
        // let lua = Instance::new();

        let script = lua.script("root.b.a");
        let interactor = test_interactor.interactor::<MyAction, _>(root);
        let value: mlua::Value = script.exec(&interactor).unwrap();

        // assert_eq!(value.as_integer().unwrap(), 3);
    }

    
}
