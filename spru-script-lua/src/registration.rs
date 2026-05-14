use std::{any::TypeId, collections::HashMap, sync::Arc};

use mlua::{UserDataFields, UserDataMethods};
use spru::item::IdT;

struct MappingFns<Storage, Action> {
    get_mapping_fn: crate::func::GetMappingFn<Storage, Action>,
    set_mapping_fn: crate::func::SetMappingFn<Storage, Action>,
    method_mapping_fn: crate::func::MethodMappingFn<Storage, Action>,
}

pub struct Registration<Storage, Action> {
    mapping_fns: HashMap<
        TypeId, 
        MappingFns<Storage, Action>,
    >,
}

impl<Storage, Action> Registration<Storage, Action> {
    pub(crate) fn new() -> Self {
        Self {
            mapping_fns: HashMap::new(),
        }
    }

    pub(crate) fn register_mappings<T: 'static>(
        &mut self, 
        get_mapping_fn: crate::func::GetMappingFn<Storage, Action>,
        set_mapping_fn: crate::func::SetMappingFn<Storage, Action>,
        method_mapping_fn: crate::func::MethodMappingFn<Storage, Action>,
    ) {
        self.mapping_fns.insert(TypeId::of::<IdT<T>>(), MappingFns {
            get_mapping_fn,
            set_mapping_fn,
            method_mapping_fn,
        });
    }
}

impl<Storage, Action> Registration<Storage, Action> {
    pub(crate) fn into_mapping_fns<'s>(self) 
        -> (
            crate::func::GetMappingRoutingFn<'s, Storage, Action>,
            crate::func::SetMappingRoutingFn<'s, Storage, Action>,
            crate::func::MethodMappingRoutingFn<'s, Storage, Action>,
            crate::func::CreateMappingRoutingFn<'s, Storage, Action>,
        )
    where
        Storage: spru::item::Storage + 's,
        Action: spru::Action<State = Storage::State>,
    {
        let Self {
            mapping_fns,
        } = self;

        let get_mapping_fns = Arc::new(mapping_fns);
        let set_mapping_fns = get_mapping_fns.clone();
        let method_mapping_fns = get_mapping_fns.clone();

        (
            // get
            Box::new(move |
                lua: &mlua::Lua,
                ledger: &spru::interactor::Ledger<Storage, Action>, 
                id: mlua::AnyUserData, 
                mapping: mlua::Value
            | -> mlua::Result<mlua::Value> 
            {

                let type_id = id.type_id()
                    .expect("IdT not registered");

                let mapping_fn = get_mapping_fns.get(&type_id)
                    .expect("IdT not mapped")
                    .get_mapping_fn;

                mapping_fn(lua, ledger, id, mapping)
            }),
            // set
            Box::new(move |
                lua: &mlua::Lua,
                ledger: &mut spru::interactor::Ledger<Storage, Action>, 
                id: mlua::AnyUserData, 
                mapping: mlua::AnyUserData,
                value: mlua::Value,
            | -> mlua::Result<()> 
            {

                let type_id = id.type_id()
                    .expect("IdT not registered");

                let mapping_fn = set_mapping_fns.get(&type_id)
                    .expect("IdT not mapped")
                    .set_mapping_fn;

                let (id, actions) = mapping_fn(lua, ledger, id, mapping, value)?;

                for action in actions {
                    ledger.enqueue_action(id, action);
                }

                ledger.flush()
                    .map_err(spru::common::error::PseudoError::into_error)
                    .map_err(mlua::Error::external)?;

                Ok(())
            }),

            // method
            Box::new(move |
                lua: &mlua::Lua,
                ledger: &mut spru::interactor::Ledger<Storage, Action>, 
                id: mlua::AnyUserData, 
                mapping: mlua::AnyUserData,
                args: mlua::MultiValue,
            | -> mlua::Result<mlua::MultiValue> 
            {

                let type_id = id.type_id()
                    .expect("IdT not registered");

                let mapping_fn = method_mapping_fns.get(&type_id)
                    .expect("IdT not mapped")
                    .method_mapping_fn;

                let (ret, id, actions) = mapping_fn(lua, ledger, id, mapping, args)?;

                for action in actions {
                    ledger.enqueue_action(id, action);
                }

                ledger.flush()
                    .map_err(spru::common::error::PseudoError::into_error)
                    .map_err(mlua::Error::external)?;

                Ok(ret)
            }),

            // create
            Box::new(move |
                lua: &mlua::Lua,
                ledger: &mut spru::interactor::Ledger<Storage, Action>, 
                mapping: mlua::AnyUserData,
                args: mlua::MultiValue,
            | -> mlua::Result<mlua::AnyUserData> 
            {
                let mapping = mapping.borrow::<crate::func::CreateFn<Action>>()?;
                let (idt_fn, action) = mapping(lua, args)?;

                let id = ledger.enqueue_create(action);

                ledger.flush()
                    .map_err(spru::common::error::PseudoError::into_error)
                    .map_err(mlua::Error::external)?;

                let idt = idt_fn(lua, id)?;

                Ok(idt)
            }),
        )
    }
}

pub struct RegistrationState<'r, T> {
    lua: &'r mlua::Lua,
    idt_registry: &'r mut mlua::UserDataRegistry<IdT<T>>,
    static_table: Option<mlua::Table>,
}

impl<'r, T> RegistrationState<'r, T> {
    pub(crate) fn new(
        lua: &'r mlua::Lua, 
        idt_registry: &'r mut mlua::UserDataRegistry<IdT<T>>,
        static_table: Option<mlua::Table>,
    ) -> mlua::Result<Self> {
        Ok(Self {
            lua,
            idt_registry,
            static_table,
        })
    }

    pub(crate) fn add_getter(&mut self, ident: &str, f: impl Fn(&mlua::Lua, &IdT<T>) -> mlua::Result<mlua::Value> + Send + 'static) -> mlua::Result<()> {
        self.idt_registry.add_field_method_get(ident, f);
        Ok(())
    }

    pub(crate) fn add_setter(&mut self, ident: &str, f: impl Fn(&mlua::Lua, &mut IdT<T>, mlua::Value) -> mlua::Result<()> + Send + 'static) -> mlua::Result<()> {
        self.idt_registry.add_field_method_set(ident, f);
        Ok(())
    }

    pub(crate) fn add_method(&mut self, ident: &str, f: impl Fn(&mlua::Lua, &IdT<T>, mlua::MultiValue) -> mlua::Result<mlua::MultiValue> + Send + 'static) -> mlua::Result<()> {
        self.idt_registry.add_method(ident, f);
        Ok(())
    }

    pub(crate) fn add_function(&mut self, ident: &str, f: impl Fn(&mlua::Lua, mlua::MultiValue) -> mlua::Result<mlua::MultiValue> + Send + 'static) -> mlua::Result<()> {
        let function = self.lua.create_function(f)?;
        if let Some(static_table) = &self.static_table {
            static_table.set(ident, function)?;
        }

        Ok(())
    }
}

pub struct RegistrationType<'r, T> {
    lua: &'r mlua::Lua,
    registry: &'r mut mlua::UserDataRegistry<T>,
    static_table: Option<mlua::Table>,
}

impl<'r, T> RegistrationType<'r, T> {
    pub(crate) fn new(
        lua: &'r mlua::Lua, 
        registry: &'r mut mlua::UserDataRegistry<T>,
        static_table: Option<mlua::Table>,
    ) -> mlua::Result<Self> {
        Ok(Self {
            lua,
            registry,
            static_table,
        })
    }

    pub(crate) fn add_getter(&mut self, ident: &str, f: impl Fn(&mlua::Lua, &T) -> mlua::Result<mlua::Value> + Send + 'static) -> mlua::Result<()> {
        self.registry.add_field_method_get(ident, f);
        Ok(())
    }

    pub(crate) fn add_method(&mut self, ident: &str, f: impl Fn(&mlua::Lua, &T, mlua::MultiValue) -> mlua::Result<mlua::MultiValue> + Send + 'static) -> mlua::Result<()> {
        self.registry.add_method(ident, f);
        Ok(())
    }

    pub(crate) fn add_function(&mut self, ident: &str, f: impl Fn(&mlua::Lua, mlua::MultiValue) -> mlua::Result<mlua::MultiValue> + Send + 'static) -> mlua::Result<()> {
        let function = self.lua.create_function(f)?;
        if let Some(static_table) = &self.static_table {
            static_table.set(ident, function)?;
        }

        Ok(())
    }

    pub(crate) fn add_eq(&mut self, f: impl Fn(&mlua::Lua, &T, mlua::Value) -> mlua::Result<bool> + Send + 'static) -> mlua::Result<()> {
        self.registry.add_meta_method(mlua::MetaMethod::Eq.name(), f);

        Ok(())
    }
}
