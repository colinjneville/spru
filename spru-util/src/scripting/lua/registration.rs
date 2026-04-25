use std::{any::TypeId, collections::HashMap};

use mlua::UserDataFields;
use spru::item::IdT;

use crate::scripting::lua;



pub(crate) struct Registration<Storage, Action> {
    mapping_fns: HashMap<
        TypeId, 
        lua::func::MappingFn<Storage, Action>
        // Box<dyn Any>
    >,
}

impl<Storage, Action> Registration<Storage, Action> {
    pub(crate) fn new() -> Self {
        Self {
            mapping_fns: HashMap::new(),
        }
    }

    pub(crate) fn register_mapping<T: 'static>(&mut self, mapping_fn: lua::func::MappingFn<Storage, Action>) {
        self.mapping_fns.insert(TypeId::of::<IdT<T>>(), mapping_fn);
    }
}

impl<Storage, Action> Registration<Storage, Action> {
    pub(crate) fn into_mapping_fn<'s>(self) 
        -> lua::func::MappingRoutingFn<'s, Storage, Action> 
    where
        Storage: 's,
        Action: 'static,
    {
        Box::new(move |
            lua: &mlua::Lua,
            ledger: &spru::interactor::Ledger<Storage, Action>, 
            id: mlua::AnyUserData, 
            mapping: mlua::AnyUserData
        | -> mlua::Result<mlua::Value> 
        {

                let type_id = id.type_id()
                    .expect("IdT not registered");

                let mapping_fn = self.mapping_fns.get(&type_id)
                    .expect("IdT not mapped");

                mapping_fn(lua, ledger, id, mapping)
        })
    }
}

pub(crate) struct RegistrationType<'r, T> {
    // lua_registration: &'r LuaRegistration<Storage, Action>,
    idt_registry: &'r mut mlua::UserDataRegistry<IdT<T>>,
}

impl<'r, T> RegistrationType<'r, T> {
    pub(crate) fn new(idt_registry: &'r mut mlua::UserDataRegistry<IdT<T>>) -> Self {
        Self {
            idt_registry,
        }
    }

    pub(crate) fn add_getter(&mut self, ident: &str, f: impl Fn(&mlua::Lua, &IdT<T>) -> mlua::Result<mlua::Value> + 'static) {
        self.idt_registry.add_field_method_get(ident, f);
    }
}