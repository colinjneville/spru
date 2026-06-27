use spru::item::IdT;

use std::sync::{Arc, RwLock, RwLockWriteGuard};

// https://rhai.rs/book/patterns/references.html
#[derive(Clone)]
pub(crate) enum StorageHandle {
    // read-only pointer to Storage
    StorageOnly(Arc<*const ()>),
    // pointer to Ledger
    Ledger(Arc<RwLock<*mut ()>>),
}

// SAFETY: rhai has Send + Sync enabled so instances can be cached and reused, so StorageHandle must
// be Send + Sync. However, we hold a write lock the entire time while StorageHandle is inserted, so
// StorageHandle can never actually be Sent.
unsafe impl Send for StorageHandle {}
unsafe impl Sync for StorageHandle {}

impl StorageHandle {
    pub fn new<'l, Storage, Action>(ledger: &mut spru::interactor::Ledger<'l, Storage, Action>) -> Self {
        let pointer = ledger as *mut spru::interactor::Ledger<'l, Storage, Action>;
        Self::Ledger(Arc::new(RwLock::new(pointer.cast())))
    }

    pub fn new_readonly<'l, Storage>(storage: &Storage) -> Self {
        let pointer = storage as *const Storage;
        Self::StorageOnly(Arc::new(pointer.cast()))
    }

    pub unsafe fn get_mut<'l, 'i, Storage, Action>(&'i mut self) 
        -> StorageAccess<'l, 'i, Storage, Action>
    {
        match self {
            StorageHandle::StorageOnly(pointer) => {
                let storage = pointer.cast::<Storage>();
                let storage = unsafe { &*storage };
                
                StorageAccess::Storage { storage }
            },
            StorageHandle::Ledger(pointer) => {
                let guard = pointer.write()
                    .expect("Ledger lock poisoned");
                
                let ledger = guard.cast::<spru::interactor::Ledger<'l, Storage, Action>>();
                let ledger = unsafe { &mut *ledger };
                StorageAccess::Ledger { _guard: guard, ledger }
            },
        }
        
    }

    pub fn from_rhai(ctx: &rhai::NativeCallContext) -> Self {
        let handle = ctx.tag()
            .expect("Ledger handle not set")
            .clone();
        
        handle.try_cast_result::<StorageHandle>()
            .expect("Expected StorageHandle")
    }
}

pub(crate) enum StorageAccess<'l, 'i, Storage, Action> {
    Storage {
        storage: &'i Storage,
    },
    Ledger {
        _guard: RwLockWriteGuard<'i, *mut ()>,
        ledger: &'i mut spru::interactor::Ledger<'l, Storage, Action>, 
    },
}

impl<'l, 'i, Storage, Action> StorageAccess<'l, 'i, Storage, Action> {
    pub(crate) fn get<T>(&self, id: IdT<T>) 
        -> Result<&T, spru::item::storage::Error> 
    where
        Storage: spru::item::Storage,
        T: spru::item::storage::Storable<Storage::State>,
    {
        match self {
            StorageAccess::Storage { storage } => storage.get(id)
                .map(spru::Item::get),
            StorageAccess::Ledger { _guard, ledger } => ledger.get(id)
                .map(|existing| existing.state()),
        }
    }

    pub(crate) fn ledger(&mut self) -> Result<&mut spru::interactor::Ledger<'l, Storage, Action>, &'static str> {
        match self {
            StorageAccess::Storage { .. } => Err("Attempted to modify state during read-only script evaluation"),
            StorageAccess::Ledger { _guard, ledger } => Ok(ledger),
        }
    }
}