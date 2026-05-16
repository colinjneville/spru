use spru::item::IdT;
use spru_script::Wrap;

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
    // [K: Eq + std::hash::Hash + mlua::FromLua, V: mlua::FromLua, S: std::hash::BuildHasher + Default] std::collections::HashMap<K, V, S>, 
    [K: Ord + mlua::FromLua, V: mlua::FromLua] std::collections::BTreeMap<K, V>,
    [T: mlua::FromLua, const N: usize] [T; N],
    [T: Eq + std::hash::Hash + mlua::FromLua, S: std::hash::BuildHasher + Default] std::collections::HashSet<T, S>,
    [T: Ord + mlua::FromLua] std::collections::BTreeSet<T>,
    // [T: mlua::FromLua] Option<T>,
    [T: mlua::FromLua] Box<[T]>,
    // [T: mlua::FromLua] Vec<T>,
}

// TODO https://docs.rs/mlua/latest/mlua/trait.FromLua.html#implementors

impl<T: FromLua> FromLua for Vec<T> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let table = <mlua::Table as mlua::FromLua>::from_lua(value, lua)?;
        let mut v = Vec::with_capacity(table.len()? as usize);
        for value in table.sequence_values() {
            v.push(T::from_lua(value?, lua)?);
        }
        Ok(v)
    }
}

impl<K: Eq + std::hash::Hash + FromLua, V: FromLua, S: std::hash::BuildHasher + Default> FromLua for std::collections::HashMap<K, V, S> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let table: mlua::Table = mlua::FromLua::from_lua(value, lua)?;
        let mut map = std::collections::HashMap::<K, V, S>::with_capacity_and_hasher(table.len()? as usize, S::default());
        for r in table.pairs::<mlua::Value, mlua::Value>() {
            let (k, v) = r?;
            let k: K = FromLua::from_lua(k, lua)?;
            let v: V = FromLua::from_lua(v, lua)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

impl<T: FromLua> FromLua for Option<T> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Nil => Ok(None),
            value => Ok(Some(T::from_lua(value, lua)?)),
        }
    }
}

impl FromLua for mlua::Value {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(value)
    }
}

impl<T: 'static> FromLua for IdT<T> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let aud = <mlua::AnyUserData as mlua::FromLua>::from_lua(value, lua)?;
        let idt = *aud.borrow::<Self>()?;
        Ok(idt)
    }
}

impl FromLua for spru::player::Id {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let aud = <mlua::AnyUserData as mlua::FromLua>::from_lua(value, lua)?;
        let pid = *aud.borrow::<Self>()?;
        Ok(pid)
    }
}

impl<T: Clone + 'static> FromLua for Wrap<T> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let aud = <mlua::AnyUserData as mlua::FromLua>::from_lua(value, lua)?;
        Ok(Wrap(aud.borrow::<T>()?.clone()))
    }
}

pub trait FromLuaMulti: Sized {
    fn from_lua_multi(values: mlua::MultiValue, lua: &mlua::Lua) -> mlua::Result<Self>;
}

impl FromLuaMulti for mlua::MultiValue {
    fn from_lua_multi(values: mlua::MultiValue, _lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(values)
    }
}

impl<T: FromLua> FromLuaMulti for T {
    fn from_lua_multi(mut values: mlua::MultiValue, lua: &mlua::Lua) -> mlua::Result<Self> {
        T::from_lua(values.pop_front().unwrap_or(mlua::Nil), lua)
    }
}
