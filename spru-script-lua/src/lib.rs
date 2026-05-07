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

use spru::item::IdT;

pub trait IntoLua {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value>;
}

// Perhaps one day...
// https://github.com/rust-lang/rfcs/issues/2758
macro_rules! forward_into_lua {
    ($( [$($generics:tt)*] $t:ty),+ $(,)?) => {
        $(
            impl <$($generics)*> IntoLua for $t {
                fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
                    mlua::IntoLua::into_lua(self, lua)
                }
            }
        )+
    }
}

// https://docs.rs/mlua/latest/mlua/trait.IntoLua.html#foreign-impls
forward_into_lua! {
    [] &str,
    [] &std::ffi::CStr,
    [] &std::ffi::OsStr,
    [] &std::path::Path,
    // [] &BStr,
    [] std::borrow::Cow<'_, str>,
    [] std::borrow::Cow<'_, std::ffi::CStr>,
    [] bool,
    [] char,
    [] f32,
    [] f64,
    [] i8,
    [] i16,
    [] i32,
    [] i64,
    [] i128,
    [] isize,
    [] u8,
    [] u16,
    [] u32,
    [] u64,
    [] u128,
    [] usize,
    [] Box<str>,
    [] std::ffi::CString,
    [] String,
    [] std::ffi::OsString,
    [] std::path::PathBuf,

    [K: Eq + std::hash::Hash + mlua::IntoLua, V: mlua::IntoLua, S: std::hash::BuildHasher] std::collections::HashMap<K, V, S>,
    [K: Ord + mlua::IntoLua, V: mlua::IntoLua] std::collections::BTreeMap<K, V>,
    [T: Clone + mlua::IntoLua] &[T],
    [T: mlua::IntoLua, const N: usize] [T; N],
    [T: Eq + std::hash::Hash + mlua::IntoLua, S: std::hash::BuildHasher] std::collections::HashSet<T, S>,
    [T: Ord + mlua::IntoLua] std::collections::BTreeSet<T>,
    [T: mlua::IntoLua] Option<T>,
    [T: mlua::IntoLua] Box<[T]>,
    [T: mlua::IntoLua] Vec<T>,
}

// TODO https://docs.rs/mlua/latest/mlua/trait.IntoLua.html#implementors

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
    ($( [$($generics:tt)*] $t:ty),+ $(,)?) => {
        $(
            impl <$($generics)*> FromLua for $t {
                fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
                    mlua::FromLua::from_lua(value, lua)
                }
            }
        )+
    }
}

// https://docs.rs/mlua/latest/mlua/trait.FromLua.html#foreign-impls
forward_from_lua! {
    [] bool,
    [] char,
    [] f32,
    [] f64,
    [] i8,
    [] i16,
    [] i32,
    [] i64,
    [] i128,
    [] isize,
    [] u8,
    [] u16,
    [] u32,
    [] u64,
    [] u128,
    [] usize,
    [] Box<str>,
    [] std::ffi::CString,
    [] String,
    [] std::ffi::OsString,
    [] std::path::PathBuf,
    [K: Eq + std::hash::Hash + mlua::FromLua, V: mlua::FromLua, S: std::hash::BuildHasher + Default] std::collections::HashMap<K, V, S>, 
    [K: Ord + mlua::FromLua, V: mlua::FromLua] std::collections::BTreeMap<K, V>,
    [T: mlua::FromLua, const N: usize] [T; N],
    [T: Eq + std::hash::Hash + mlua::FromLua, S: std::hash::BuildHasher + Default] std::collections::HashSet<T, S>,
    [T: Ord + mlua::FromLua] std::collections::BTreeSet<T>,
    [T: mlua::FromLua] Option<T>,
    [T: mlua::FromLua] Box<[T]>,
    [T: mlua::FromLua] Vec<T>,
}

// TODO https://docs.rs/mlua/latest/mlua/trait.FromLua.html#implementors

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
    #[spru_script::script(include = [XFun])]
    struct X<T: 'static> {
        #[get]
        #[set]
        a: T,
        // b: IdT<X>,
    }

    #[spru_script::script(partial = XFun)]
    impl<T> X<T> {
        #[create]
        fn new((value, _i): (T, i32)) -> spru_util::cloned::Create<X<T>> {
            spru_util::cloned::create(Self { a: value })
        }

        #[method]
        fn destroy(&self) -> ((), spru_util::cloned::Destroy<X<T>>) {
            ((), spru_util::cloned::destroy())
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[spru_script::script]
    struct Y {
        #[get]
        #[set]
        b: IdT<X<i32>>,
        #[get]
        #[set]
        c: IdT<X<i64>>,
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
            local x2 = X[i32].new(5, 7)
            root.b = x2
            print(root.b.a)
            print(root.b:exists())
            local aa = root.b.a
            root.b:destroy()
            print(root.b:exists())
            return aa
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
