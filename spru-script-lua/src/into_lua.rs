use spru::item::IdT;
use spru_script::Wrap;

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

    // TODO These need to hand-written to use `T: crate::IntoLua`...
    [K: Eq + std::hash::Hash + mlua::IntoLua, V: mlua::IntoLua, S: std::hash::BuildHasher] std::collections::HashMap<K, V, S>,
    [K: Ord + mlua::IntoLua, V: mlua::IntoLua] std::collections::BTreeMap<K, V>,
    [T: Clone + mlua::IntoLua] &[T],
    [T: mlua::IntoLua, const N: usize] [T; N],
    [T: Eq + std::hash::Hash + mlua::IntoLua, S: std::hash::BuildHasher] std::collections::HashSet<T, S>,
    [T: Ord + mlua::IntoLua] std::collections::BTreeSet<T>,
    // [T: mlua::IntoLua] Option<T>,
    [T: mlua::IntoLua] Box<[T]>,
    // [T: mlua::IntoLua] Vec<T>,
}

// TODO https://docs.rs/mlua/latest/mlua/trait.IntoLua.html#implementors

struct DeferredIntoLua<F>(F);

fn deferred_into_lua<T: IntoLua>(value: T) -> DeferredIntoLua<impl FnOnce(&mlua::Lua) -> mlua::Result<mlua::Value>> {
    DeferredIntoLua(
        move |lua: &mlua::Lua| IntoLua::into_lua(value, lua)
    )
}

impl<F: FnOnce(&mlua::Lua) -> mlua::Result<mlua::Value>> mlua::IntoLua for DeferredIntoLua<F> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        (self.0)(lua)
    }
}

impl IntoLua for mlua::Value {
    fn into_lua(self, _lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(self)
    }
}

impl<T: IntoLua> IntoLua for Vec<T> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Table(lua.create_sequence_from(
            self.into_iter().map(deferred_into_lua)
        )?))
    }
}

impl<T: IntoLua> IntoLua for Option<T> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        match self {
            Some(value) => T::into_lua(value, lua),
            None => Ok(mlua::Value::Nil),
        }
    }
}

impl<T: 'static> IntoLua for IdT<T> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        mlua::IntoLua::into_lua(lua.create_any_userdata(self)?, lua)
    }
}

impl IntoLua for spru::player::Id {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        mlua::IntoLua::into_lua(lua.create_any_userdata(self)?, lua)
    }
}

impl<T: mlua::MaybeSend + 'static> IntoLua for Wrap<T> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let aud = lua.create_any_userdata(self.0)?;
        Ok(mlua::Value::UserData(aud))
    }
}

pub trait IntoLuaMulti: Sized {
    fn into_lua_multi(self, lua: &mlua::Lua) -> mlua::Result<mlua::MultiValue>;
}

impl IntoLuaMulti for mlua::MultiValue {
    fn into_lua_multi(self, _lua: &mlua::Lua) -> mlua::Result<mlua::MultiValue> {
        Ok(self)
    }
}

impl<T: IntoLua> IntoLuaMulti for T {
    fn into_lua_multi(self, lua: &mlua::Lua) -> mlua::Result<mlua::MultiValue> {
        let mut v = mlua::MultiValue::with_capacity(1);
        v.push_back(self.into_lua(lua)?);
        Ok(v)
    }
}