pub(crate) struct Ledger<'m, 'l, 'f, Storage, Action> {
    ledger: &'m mut spru::interactor::Ledger<'l, Storage, Action>, 
    get_mapping_fn: &'f crate::func::GetMappingRoutingFn<'f, Storage, Action>,
    set_mapping_fn: &'f crate::func::SetMappingRoutingFn<'f, Storage, Action>,
    method_mapping_fn: &'f crate::func::MethodMappingRoutingFn<'f, Storage, Action>,
    create_mapping_fn: &'f crate::func::CreateMappingRoutingFn<'f, Storage, Action>,
}

impl<'m, 'l, 'f, Storage, Action> std::fmt::Debug for Ledger<'m, 'l, 'f, Storage, Action> 
where 
    spru::interactor::Ledger<'l, Storage, Action>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            ledger,
            get_mapping_fn: _get_mapping_fn,
            set_mapping_fn: _set_mapping_fn,
            method_mapping_fn: _method_mapping_fn,
            create_mapping_fn: _create_mapping_fn,
        } = self;

        f.debug_struct("LuaLedger")
            .field("ledger", ledger)
            .field("get_mapping_fn", &())
            .field("set_mapping_fn", &())
            .field("method_mapping_fn", &())
            .field("create_mapping_fn", &())
            .finish()
    }
}


impl<'m, 'l, 'f, Storage: spru::item::Storage, Action: spru::Action<State = Storage::State>> Ledger<'m, 'l, 'f, Storage, Action> {
    pub(crate) fn new(
        ledger: &'m mut spru::interactor::Ledger<'l, Storage, Action>, 
        get_mapping_fn: &'f crate::func::GetMappingRoutingFn<'f, Storage, Action>,
        set_mapping_fn: &'f crate::func::SetMappingRoutingFn<'f, Storage, Action>,
        method_mapping_fn: &'f crate::func::MethodMappingRoutingFn<'f, Storage, Action>,
        create_mapping_fn: &'f crate::func::CreateMappingRoutingFn<'f, Storage, Action>,
    ) 
        -> Self 
    {
        Self {
            ledger,
            get_mapping_fn,
            set_mapping_fn,
            method_mapping_fn,
            create_mapping_fn,
        }
    }
}

impl<Storage, Action> mlua::UserData for Ledger<'_, '_, '_, Storage, Action> 
where 
    Storage: spru::item::Storage,
    Action: spru::Action<State = Storage::State> + 'static,
{
    fn add_methods<'lua, M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method::<_, (mlua::AnyUserData, mlua::Value), mlua::Value>(crate::key::LEDGER_METHOD_GET, 
            |lua, this, (id, mapping)| {
                (this.get_mapping_fn)(lua, &this.ledger, id, mapping)
            }
        );

        methods.add_method_mut::<_, (mlua::AnyUserData, mlua::AnyUserData, mlua::Value), ()>(crate::key::LEDGER_METHOD_SET, 
            |lua, this, (id, mapping, value)| {
                (this.set_mapping_fn)(lua, &mut this.ledger, id, mapping, value)
            }
        );

        methods.add_method_mut::<_, (mlua::AnyUserData, mlua::AnyUserData, mlua::MultiValue), mlua::MultiValue>(crate::key::LEDGER_METHOD_METHOD, 
            |lua, this, (id, mapping, args)| {
                (this.method_mapping_fn)(lua, &mut this.ledger, id, mapping, args)
            }
        );

        methods.add_method_mut::<_, (mlua::AnyUserData, mlua::MultiValue), mlua::AnyUserData>(crate::key::LEDGER_METHOD_CREATE, 
            |lua, this, (mapping, args)| {
                (this.create_mapping_fn)(lua, &mut this.ledger, mapping, args)
            }
        );
    }
}
