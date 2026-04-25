use crate::scripting::lua;

pub(crate) struct Ledger<'l, Storage, Action> {
    ledger: &'l spru::interactor::Ledger<'l, Storage, Action>, 
    mapping_fn: &'l lua::func::MappingRoutingFn<'l, Storage, Action>,
}

impl<'l, Storage, Action> std::fmt::Debug for Ledger<'l, Storage, Action> 
where 
    spru::interactor::Ledger<'l, Storage, Action>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuaLedger")
            .field("ledger", &self.ledger)
            .field("mapping_fn", &())
            .finish()
    }
}


impl<'l, Storage: spru::item::Storage, Action: spru::Action<State = Storage::State>> Ledger<'l, Storage, Action> {
    pub(crate) fn new(ledger: &'l spru::interactor::Ledger<'l, Storage, Action>, mapping_fn: &'l lua::func::MappingRoutingFn<'l, Storage, Action>) -> Self {
        Self {
            ledger,
            mapping_fn,
        }
    }
}

impl<Storage, Action> mlua::UserData for Ledger<'_, Storage, Action> 
where 
    Storage: spru::item::Storage,
    Action: spru::Action<State = Storage::State> + 'static,
{
    fn add_methods<'lua, M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut::<_, (mlua::AnyUserData, mlua::AnyUserData), mlua::Value>(lua::key::LEDGER_METHOD_GET, 
            |lua, this, (id, mapping)| {
                (this.mapping_fn)(lua, &this.ledger, id, mapping)
            }
        );
    }
}
