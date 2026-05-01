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
// pub mod script;
// pub use script::Script;

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

pub trait FromLua: Sized {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self>;
}

// Perhaps one day...
// https://github.com/rust-lang/rfcs/issues/2758
macro_rules! forward_from_lua {
    ($(<>)? $($t:ty),+ $(,)?) => {
        $(
            impl FromLua for $t {
                fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
                    mlua::FromLua::from_lua(value, lua)
                }
            }
        )+
    }
}

// https://docs.rs/mlua/latest/mlua/trait.FromLua.html#foreign-impls
forward_from_lua! {
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

impl<T: 'static> FromLua for IdT<T> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let aud = <mlua::AnyUserData as mlua::FromLua>::from_lua(value, lua)?;
        let idt = *aud.borrow::<Self>()?;
        Ok(idt)
    }
}

#[cfg(test)]

mod test {
    use tagset::tagset;

    use spru::item::IdT;

    use spru_util::cloned;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct X<T> {
        a: T,
        // b: IdT<X>,
    }

    impl<T: Copy + 'static, State, Action, Registry> spru_script::ScriptableType<State, Action, Registry> for X<T>
    where 
        State: spru::State,
        Action: spru::Action +
            From<cloned::Create<Self>> +
            From<cloned::Update<Self>> +
            From<cloned::Destroy<Self>> +
            ,
        Registry: 
            spru_script::RegistryCreate<State, Action, cloned::Create<Self>, Self, T> +
            spru_script::RegistryGetter<State, Action, Self, T> +
            spru_script::RegistrySetter<State, Action, Self, T> +
            spru_script::RegistryMethod<State, Action, Self, T, T> +
            spru_script::RegistryMethod<State, Action, Self, (), ()> +
            ,
    {
        fn register<Storage>(registry: &Registry, registration: &mut Registry::MemberRegistration<'_, Storage, Self>)
             -> Result<(), Registry::Error> 
        where
            Storage: spru::item::Storage<State = State>,
        {
            registry.register_get(registration, "a", |this| this.a)?;
            registry.register_set(registration, "a", |_this, value| vec![cloned::update(X { a: value }).into()])?;
            // registry.register_method(registration, "multiplier", |this, args| {
            //     let product = this.a * args;
            //     (product, vec![cloned::update(X { a: product}).into()])
            // })?;
            registry.register_method(registration, "delete", |_this, ()| {
                ((), vec![cloned::destroy().into()])
            })?;
            registry.register_create(registration, "new", |args| {
                cloned::create(X { a: args })
            })?;
            Ok(())
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct Y {
        b: IdT<X<i32>>,
        c: IdT<X<i64>>,
    }

    impl<State, Action, Registry> spru_script::ScriptableType<State, Action, Registry> for Y
    where 
        State: spru::State,
        Action: spru::Action +
            From<cloned::Update<Y>> +
            ,
        Registry: 
            spru_script::RegistryGetter<State, Action, Self, IdT<X<i32>>> +
            spru_script::RegistrySetter<State, Action, Self, IdT<X<i32>>> +
            spru_script::RegistryGetter<State, Action, Self, IdT<X<i64>>> +
            spru_script::RegistrySetter<State, Action, Self, IdT<X<i64>>> +
            ,
    {
        fn register<Storage>(registry: &Registry, registration: &mut Registry::MemberRegistration<'_, Storage, Self>) 
            -> Result<(), Registry::Error> 
        where
            Storage: spru::item::Storage<State = State>,
        {
            registry.register_get(registration, "b", |this| this.b)?;
            registry.register_set(registration, "b", |this, value| vec![cloned::update(Self {b: value, .. this.clone() }).into()])?;
            registry.register_get(registration, "c", |this| this.c)?;
            registry.register_set(registration, "c", |this, value| vec![cloned::update(Self {c: value, .. this.clone() }).into()])?;
            Ok(())
        }
    }

    #[tagset(impl tagset::proxy::serde::Serialize)]
    #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
    #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
    #[tagset(impl spru::State)]
    #[tagset(impl<Action, Registry> spru_script::ScriptableState<Action, Registry>)]
    #[tagset(derive(Debug))]
    #[tagset(X<i32>)]
    #[tagset(X<i64>)]
    #[tagset(Y)]
    struct MyState;

    #[tagset(impl spru::Action {
        type State = MyState;
    })]
    #[tagset(impl tagset::proxy::serde::Serialize)]
    #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
    #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
    #[tagset(derive(Debug, Clone))]
    #[tagset(include(cloned::Actions<X<i32>>))]
    #[tagset(include(cloned::Actions<X<i64>>))] 
    #[tagset(include(cloned::Actions<Y>))]
    struct MyAction;


    #[test]
    fn register_type() {
        use spru_script::Language as _;

        let storage = spru_util::storage::Standalone::<MyState>::new();

        let lua = crate::Lua::<MyState, MyAction>::new();

        // let game_init = game_init::GameInit::new(lua.script("
        //     root.b.a
        // "));

        let mut test_interactor = spru::interactor::test_util::TestInteractor::new(storage);

        let mut interactor = test_interactor.interactor::<MyAction, _>(());
        let x32 = interactor
            .create(cloned::create(X { a: 3i32 }));
        let x64 = interactor
            .create(cloned::create(X { a: 4i64 }));
        let y = interactor
            .create(cloned::create(Y { b: x32.id(), c: x64.id() }));
        let root = y.id();

        interactor.flush().unwrap();

        
        // let interaction = interaction::Interaction::<MyState, MyAction, IdT<Y>, i32>
        //     ::new(lua, "root.b.a".to_string());

        // spru::Server::init(game_init, player_init, reaction)


        // spru::Interaction::apply(&interaction, &mut interactor)
        //     .unwrap();

        // let mut interactor = test_interactor.interactor::<MyAction, _>(());
        // let x = interactor
        //     .create(cloned::create(X { a: 3 }));
        // let y = interactor
        //     .create(cloned::create(Y { b: x.id() }));
        // let root = y.id();

        // interactor.flush().unwrap();
        
        // let lua = Instance::new();

        let script = "
            local c = root.b:multiplier(4)
            root.b:multiplier(c)
            return root.b.a
        ";

        let script = "
            local x2 = X[i32].new(5)
            root.b = x2
            print(root.b:exists())
            root.b:delete()
            print(root.b:exists())
            return 5
        ";

        let mut interactor = test_interactor.interactor::<MyAction, _>(root);
        let value: mlua::Value = lua.exec(&mut interactor, script).unwrap();

        // let script2 = lua.script("
        //     return root.b.a
        // ");

        // let value2: mlua::Value = script2.exec(&mut interactor).unwrap();
        println!("{}", value.as_integer().unwrap());

        // assert_eq!(value.as_integer().unwrap(), (3 * 4) * (3 * 4));
        assert_eq!(value.as_integer().unwrap(), 5);
    }

    
}
