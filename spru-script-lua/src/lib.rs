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

mod from_lua;
pub use from_lua::{FromLua, FromLuaMulti};
pub(crate) mod func;
mod into_lua;
pub use into_lua::{IntoLua, IntoLuaMulti};
mod instance;
pub use instance::Lua;
pub(crate) mod key;
mod ledger;
use ledger::Ledger;
mod registration;
use registration::{Registration, RegistrationState, RegistrationType};
pub mod registry;
use registry::Registry;

macro_rules! lua_multi {
    () => { 
        impl IntoLuaMulti for () {
            fn into_lua_multi(self, _lua: &mlua::Lua) -> mlua::Result<mlua::MultiValue> {
                Ok(mlua::MultiValue::new())
            }
        }

        impl FromLuaMulti for () {
            fn from_lua_multi(_values: mlua::MultiValue, _lua: &mlua::Lua) -> mlua::Result<Self> {
                Ok(())
            }
        }
    };
    ($n:tt $first:ident $($nn:tt $rest:ident)*) => {
        impl<$first, $($rest),*> IntoLuaMulti for ($first, $($rest),*) 
        where
            $first: IntoLua,
            $($rest: IntoLua),*
        {
            fn into_lua_multi(self, lua: &mlua::Lua) -> mlua::Result<mlua::MultiValue> {
                let mut multi = mlua::MultiValue::new();
                multi.push_back(IntoLua::into_lua(self.$n, lua)?);
                $(
                    multi.push_back(IntoLua::into_lua(self.$nn, lua)?);
                )*

                Ok(multi)
            }
        }

        impl<$first, $($rest),*> FromLuaMulti for ($first, $($rest),*) 
        where
            $first: FromLua,
            $($rest: FromLua),*
        {
            #[allow(non_snake_case)]
            fn from_lua_multi(mut values: mlua::MultiValue, lua: &mlua::Lua) -> mlua::Result<Self> {
                let first = FromLua::from_lua(values.pop_front().unwrap_or(mlua::Nil), lua)?;
                $(let $rest = FromLua::from_lua(values.pop_front().unwrap_or(mlua::Nil), lua)?;)*
                Ok((
                    first,
                    $($rest),*
                ))
            }
        }
       
        lua_multi!($($nn $rest)*);
    };
}

lua_multi!(15 P 14 O 13 N 12 M 11 L 10 K 9 J 8 I 7 H 6 G 5 F 4 E 3 D 2 C 1 B 0 A);

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
    #[tagset(impl<Action, Registry> spru_script::Scriptable<Action, Registry>)]
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

        panic!("TODO TestInteractor needs to be made to work with spru_script_lua::Context");


        // let value: mlua::Value = lua.exec(&mut interactor, script, mlua::Nil).unwrap();

        // // let value2: mlua::Value = script2.exec(&mut interactor).unwrap();
        // println!("{}", value.as_integer().unwrap());

        // // assert_eq!(value.as_integer().unwrap(), (3 * 4) * (3 * 4));
        // assert_eq!(value.as_integer().unwrap(), 5);
    }
}
