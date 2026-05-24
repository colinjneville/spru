pub(crate) type GetMappingFn<Storage, Action> = 
    fn(
        &spru::interactor::Ledger<Storage, Action>,
        // IdT
        rhai::Dynamic,
        // GetFn<T> | Nil (existence check)
        rhai::Dynamic,
    ) -> rhai::plugin::RhaiResult;