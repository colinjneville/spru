mod context;
pub use context::Context;
mod dynamic;
mod instance;
pub use instance::{Rhai, RhaiInstance};
pub(crate) mod key;
mod output;
pub use output::Output;
mod settings;
pub use settings::Settings;
mod storage_handle;
#[doc(hidden)]
pub mod traits;

type RhaiResult<T> = Result<T, rhai::EvalAltResult>;

use spru_script::ScriptablePath;

#[macro_export]
macro_rules! _rhai {
    (<$storage:ty, $action:ty> $registration:ident {
        $(
            $macro_path:path => $ty:path $(as $type_alias:path)?;
        )*
    } ) => {
        #[allow(unused_imports)]
        use spru_script_rhai::traits::{
            RegisterStateMemberNoop as _, RegisterStateMember as _,
            RegisterStateNoop as _, RegisterState as _, 
        };

        $(
            $macro_path!(<$storage, $action> $registration => $ty $(as $type_alias)?);
        )*
    };
    ($registration:ident {
        $(
            $macro_path:path => $ty:path $(as $type_alias:path)?;
        )*
    } ) => {
        #[allow(unused_imports)]
        use spru_script_rhai::traits::{
            RegisterStatelessMemberNoop as _, RegisterStatelessMember as _,
            RegisterStatelessNoop as _, RegisterStateless as _, 
        };

        $(
            $macro_path!($registration => $ty $(as $type_alias)?);
        )*
    };
}
pub use _rhai as rhai;




macro_rules! expand_foreach {
    ($dollar:tt { $($pre:tt)* } [$($t:ident),*] { $($post:tt)* }) => {
        macro_rules! _expand_foreach {
            ($dollar t:ident) => {
                $($pre)* $dollar t $($post)*
            };
        }
        $(_expand_foreach!($t);)*
    };
}
pub(crate) use expand_foreach;



pub struct Registration<'r> {
    rhai: &'r mut rhai::Engine,
    globals: rhai::Map,
    statics_maps: rhai::Map,
}

impl<'r> Registration<'r> {
    pub fn new(rhai: &'r mut rhai::Engine) -> Self {
        Self {
            rhai,
            globals: rhai::Map::new(),
            statics_maps: rhai::Map::new(),
        }
    }

    pub fn type_registration<'r2>(&'r2 mut self, type_path: Option<ScriptablePath>) -> MemberRegistration<'r, 'r2> {
        let statics_map = type_path.map(|tp| (tp, rhai::Map::new()));
        MemberRegistration {
            registration: self, 
            statics_map,
        }
    }

    pub fn apply(self) -> rhai::Scope<'static> {
        let mut scope = rhai::Scope::new();
        for (base_segment, map) in self.globals {
            scope.push(base_segment, map);
        }
        scope.push(&*crate::key::GLOBAL_TYPE, self.statics_maps);

        scope
    }
}


pub struct MemberRegistration<'r1, 'r2> {
    registration: &'r2 mut Registration<'r1>,
    statics_map: Option<(ScriptablePath, rhai::Map)>,
}

impl MemberRegistration<'_, '_> {
    pub fn apply(self) {
        if let Some((type_path, statics_map)) = self.statics_map {
            let mut key = String::new();
            write_scriptable_path(&mut key, &type_path);
            self.registration.statics_maps.insert(key.into(), statics_map.into());
        }
    }
}

fn write_scriptable_path(s: &mut String, type_path: &ScriptablePath) {
    use std::fmt::Write as _;

    let &ScriptablePath(path, type_args) = type_path;

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
