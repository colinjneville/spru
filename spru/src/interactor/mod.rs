use std::{cell::{self, Cell, RefCell}, collections::{HashMap, VecDeque}, ops};

use crate::{action, interaction, item::{self, lookup, IdT}, log, player, reaction, record::{self, Records}, Item};

#[macro_export]
macro_rules! follow {
    (
        $first_get:ident => $first_next_id:expr
        $(, $get:ident => $next_id:expr)*
        $(,)? 
    ) => {
        'result: {
            let next_id = $first_next_id;
            let mut get = match $first_get.follow(next_id) {
                Ok(get) => get,
                Err(e) => break 'result Err(e),
            };

            $(
                let mut $get = get;
                let next_id = $next_id;

                let mut get = match $get.follow(next_id) {
                    Ok(get) => get,
                    Err(e) => break 'result Err(e),
                };
            )*
            Ok(get)
        }
    };
}
use derive_where::derive_where;
pub use follow;

// #[macro_export]
// macro_rules! update {
//     ($get:expr, $update:expr) => {
//         {
//             let get = $get;
//             let update = $update;
//             let ret = ::$crate::action::UpdateReturn::return_value(&update, get.get());
//             get.update(update)
//                 .map(|_| ret)
//         }
//     };
// }
// pub use update;

// #[derive(Debug)]
// enum ItemTracking<Action> {
//     // We only read the item. The version must match to ensure the read remains
//     // the same, but no undo is needed
//     Read(item::Version),
//     // Log is generated, but items are not modified yet. Not implemented until 
//     // there is a clear use case
//     Deferred(std::convert::Infallible),
//     Modified(Modified<Action>),
// }

#[derive(Debug)]
struct ItemStatus<Action> {
    version_change: item::version::Change,
    flushed_do: Vec<Action>,
    flushed_undo: Vec<Action>,
    pending_do: RefCell<VecDeque<Action>>,
}

impl<Action> ItemStatus<Action> {
    fn pending() -> Self {
        Self {
            version_change: item::version::Change::create(),
            flushed_do: vec![],
            flushed_undo: vec![],
            pending_do: RefCell::default(),
        }
    }

    fn existing(version: item::Version) -> Self {
        Self {
            version_change: item::version::Change::update(version),
            flushed_do: vec![],
            flushed_undo: vec![],
            pending_do: RefCell::default(),
        }
    }

    fn enqueue(&self, action: Action) {
        self.pending_do.borrow_mut()
            .push_back(action);
    }

    fn flush<Lookup>(&mut self, id: item::Id, lookup: &mut Lookup) 
        -> record::Result<()>
    where 
        Action: crate::Action<Lookup, Undo = Action>,
    {
        while let Some(pending_do) = self.pending_do.get_mut().pop_back() {
            let context = action::Context::new(lookup, id, self.version_change);
            let undo = pending_do.apply(context)?;
            if let Some(undo) = undo {
                self.flushed_do.push(pending_do);
                self.flushed_undo.push(undo);
            }
        }

        Ok(())
    }

    fn is_flushed(&self) -> bool {
        self.pending_do.borrow().is_empty()
    }

    fn revert<Lookup>(self, id: item::Id, lookup: &mut Lookup)
        -> action::Result<()>
    where 
        Action: crate::Action<Lookup>,
    {
        for undo in self.flushed_undo.into_iter().rev() {
            let context = action::Context::new(lookup, id, self.version_change);
            let _redo = undo.apply(context)?;
        }

        Ok(())
    }

    fn expected_version(&self) -> item::Version {
        self.version_change.before
    }

    fn into_packed(self, id: item::Id) -> Option<(record::Packed<Action>, record::Packed<Action>)> {
        if !self.is_flushed() {
            panic!("Interactor must be flushed first");
        }

        let mut flushed_do = self.flushed_do.into_iter();
        if let Some(first_do) = flushed_do.next() {
            let mut packed_do = record::Packed::new(id, self.version_change, first_do);
            while let Some(do_action) = flushed_do.next() {
                packed_do.append(do_action);
            }

            let mut flushed_undo = self.flushed_undo.into_iter();
            let first_undo = flushed_undo.next().expect("do and undo must have same length");
            let mut packed_undo = record::Packed::new(id, self.version_change.undo(), first_undo);
            while let Some(undo_action) = flushed_undo.next() {
                packed_undo.append(undo_action);
            }

            Some((packed_do, packed_undo))
        } else {
            None
        }
    }
}

#[derive(Debug)]
#[derive_where(Default)]
struct ItemsStatus<Action> {
    items: RefCell<HashMap<item::Id, ItemStatus<Action>>>,
}

impl<Action> ItemsStatus<Action> {
    fn register_read(&self, id: item::Id, version: item::Version) {
        self.items.borrow_mut()
            .entry(id)
            .or_insert(ItemStatus::existing(version));
    }

    fn enqueue_create(&self, id: item::Id, action: Action) {
        self.items.borrow_mut()
            .entry(id)
            .or_insert(ItemStatus::pending())
            .enqueue(action);
    }

    fn enqueue(&self, id: item::Id, action: Action) {
        self.items.borrow_mut()
            .get_mut(&id)
            .expect("id must be added as read first")
            .enqueue(action);
    }

    fn flush<Lookup>(&mut self, lookup: &mut Lookup)
        -> record::Result<()>
    where 
        Action: crate::Action<Lookup, Undo = Action>,
    {
        for (&id, item) in self.items.get_mut() {
            item.flush(id, lookup)?;
        }
        
        Ok(())
    }

    fn revert<Lookup>(self, lookup: &mut Lookup)
        -> action::Result<()>
    where 
        Action: crate::Action<Lookup, Undo = Action>,
    {
        for (id, item) in self.items.into_inner() {
            item.revert(id, lookup)?;
        }

        Ok(())
    }

    fn expected_versions(&self) -> item::version::Expected {
        let mut versions = vec![];
        for (&item_id, item) in self.items.borrow().iter() {
            versions.push((item_id, item.expected_version()));
        }

        item::version::Expected::new(versions.into_iter())
    }

    fn into_packed(self) -> (Vec<record::Packed<Action>>, Vec<record::Packed<Action>>) {
        let mut packed_do = vec![];
        let mut packed_undo = vec![];
        for (item_id, item) in self.items.into_inner() {
            if let Some((item_do, item_undo)) = item.into_packed(item_id) {
                packed_do.push(item_do);
                packed_undo.push(item_undo);
            }
        }

        (packed_do, packed_undo)
    }
}

#[derive(Debug)]
struct Inner<'l, Lookup, Action> {
    lookup: &'l mut Lookup,
    items_status: ItemsStatus<Action>,
    reservation: &'l item::id::Reservation,
}

#[derive(Debug)]
pub(crate) struct Interactor<'l, Lookup, Action, Context, Output> {
    inner: Inner<'l, Lookup, Action>,
    context: Context,
    output: RefCell<Output>,
}

impl<'l, Lookup, Action, Context, Output> Interactor<'l, Lookup, Action, Context, Output> {
    pub(crate) fn new(lookup: &'l mut Lookup, reservation: &'l item::id::Reservation, context: Context) 
        -> Self 
    where 
        Output: Default,
    {
        Self {
            inner: Inner {
                lookup,
                items_status: ItemsStatus::default(),
                reservation,
            },
            context,
            output: RefCell::new(Output::default())
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    // Hack for player init that should be refactored at some point
    pub(crate) fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    pub fn create<Create>(&self, create: Create)
        -> Pending<Lookup, Action, Create::T>
    where 
        Create:
            Into<Action> + 
            action::Create,
    {
        let item_id = self.inner.reservation.claim_id()
            .unwrap_or_else(|| unimplemented!("Out of ids"));
        self.inner.items_status.enqueue_create(item_id, create.into());

        Pending {
            inner: &self.inner,
            item_id: item_id.force_type(),
        }
    }

    pub fn get<T>(&self, id: IdT<T>)
        -> Result<Existing<Lookup, Action, T>, lookup::Error>
    where
        Lookup: lookup::Lookup<T>,
    {
        let item = self.inner.lookup.lookup(id)?;
        self.inner.items_status.register_read(id.untyped(), item.version());

        Ok(Existing {
            inner: &self.inner,
            item,
        })
    }

    pub fn enqueue_trigger(&self, trigger: Output::Trigger)
    where 
        Output: EnqueueTrigger
    {
        self.output.borrow_mut().enqueue_trigger(trigger);
    }

    pub fn set_game_outcome(&self, game_outcome: Output::GameOutcome)
    where 
        Output: SetGameOutcome
    {
        self.output.borrow_mut().set_game_outcome(game_outcome);
    }

    pub fn flush(&mut self)
        -> record::Result<()>
    where 
        Action: crate::Action<Lookup, Undo = Action>,
    {
        self.inner.items_status.flush(self.inner.lookup)
            .map_err(Into::into)
    }

    pub(crate) fn revert(self, initial: record::Error)
        -> log::Result<record::Error>
    where 
        Action: crate::Action<Lookup, Undo = Action>,
    {
        match self.inner.items_status.revert(self.inner.lookup) {
            Ok(()) => Ok(initial),
            Err(fatal) => {
                Err(log::Error::Revert(log::RevertError {
                    initial: Some(initial.into()),
                    fatal: fatal.into(),
                }))
            }
        }
    }

    // Interactor must be flushed before calling `complete`
    pub(crate) fn complete(self) -> Complete<Action, Output> {
        let Self {
            inner: Inner {
                lookup: _lookup,
                items_status,
                reservation: _reservation,
            },
            context: _context,
            output,
        } = self;

        let expected_versions = items_status.expected_versions();
        let (do_records, undo_records) = items_status.into_packed();

        Complete {
            expected_versions,
            do_records,
            undo_records,
            output: output.into_inner(),
        }
    }
}

pub(crate) struct Complete<Action, Output> {
    pub(crate) expected_versions: item::version::Expected,
    pub(crate) do_records: Vec<record::Packed<Action>>,
    pub(crate) undo_records: Vec<record::Packed<Action>>,
    pub(crate) output: Output,
}

#[doc(hidden)]
pub trait EnqueueTrigger {
    type Trigger;

    fn enqueue_trigger(&mut self, trigger: Self::Trigger);
}

#[doc(hidden)]
pub trait SetGameOutcome {
    type GameOutcome;

    fn set_game_outcome(&mut self, game_outcome: Self::GameOutcome);
}

#[derive(Debug)]
pub struct Pending<'i, Lookup, Action, T> {
    inner: &'i Inner<'i, Lookup, Action>,
    item_id: IdT<T>,
}

impl<'i, Lookup, Action, T> Pending<'i, Lookup, Action, T> {
    pub fn update<Update>(&self, update: Update) 
        -> &Self
    where 
        Update:
            Into<Action> + 
            action::Update<T = T>,
    {
        self.inner.items_status.enqueue(self.item_id.untyped(), update.into());
        self
    }
}

#[derive(Debug)]
pub struct Existing<'i, Lookup, Action, T> {
    inner: &'i Inner<'i, Lookup, Action>,
    item: &'i Item<T>,
}

impl<'i, Lookup, Action, T> Existing<'i, Lookup, Action, T> {
    pub fn update<Update>(&self, update: Update) 
        -> &Self
    where 
        Update:
            Into<Action> + 
            action::Update<T = T>,
    {
        self.inner.items_status.enqueue(self.item.id().untyped(), update.into());
        self
    }

    pub fn destroy<Destroy>(&self, destroy: Destroy) 
    where 
        Destroy:
            Into<Action> + 
            action::Destroy<T = T>,
    {
        self.inner.items_status.enqueue(self.item.id().untyped(), destroy.into());
    }
}

impl<'i, Lookup, Action, T> ops::Deref for Existing<'i, Lookup, Action, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.item.get()
    }
}

// #[derive(Debug)]
// pub(crate) struct Get<'l, 'i, Lookup, Action, Context, Output, T> {
//     interactor: &'i Interactor<'l, Lookup, Action, Context, Output>,
//     item_id: IdT<T>,
// }

// impl<'l, 'i, Lookup, Action, Context, Output, T> Get<'l, 'i, Lookup, Action, Context, Output, T> {
//     pub fn get(&self) -> Result<&T, lookup::Error> {

//     }

//     pub(crate) fn insert_action(&self, version: item::Version, do_action: Action, undo_action: Action) {
//         self.interactor.mutable.borrow_mut().insert_action(self.item_id.untyped(), version, do_action, undo_action);
//     }
// }

// impl<'l, Lookup, Action, Context> Interactor<'l, Lookup, Action, Context> {
//     pub(crate) fn new(lookup: &'l Lookup, reservation: &'l item::id::Reservation, context: Context) -> Self {
//         Self {
//             lookup,
//             context,
//             reservation,
//             versioned: RefCell::new(HashMap::new()),
//         }
//     }

//     pub fn lookup(&self) -> &'l Lookup {
//         self.lookup
//     }

//     pub(crate) fn expected_versions(&self) -> item::version::Expected {
//         let versioned = self.versioned.borrow();
//         let iter = versioned.iter()
//             .map(|(&id, versioned)| (id, versioned.expected()));
//         item::version::Expected::new(iter)
//     }

//     pub(crate) fn take_records(&mut self) -> Records<Action> {
//         let mut records = Records::new();
//         for (_, versioned) in self.versioned.borrow_mut().drain() {
//             if let Versioned::Record(packed) = versioned {
//                 records.push(packed);
//             }
//         }

//         records
//     }

//     pub(crate) fn into_context(self) -> Context {
//         self.context
//     }

//     pub fn get<'i, T>(&'i self, id: IdT<T>) -> Result<Get<'l, 'i, Lookup, Action, Context, T>, lookup::Error> 
//     where
//         Lookup: lookup::OfType<T>,
//     {
//         let item: &Item<T> = self.lookup.lookup(id)?;

//         Ok(Get::new(self, item))
//     }

//     pub fn create<Create>(&self, create: Create) -> Result<IdT<Create::T>, lookup::Error> 
//     where 
//         Lookup: item::Lookup,
//         Create: 
//             Into<Action> + 
//             action::Create,
//     {
//         let id = self.reservation.claim_id().unwrap_or_else(|| unimplemented!());

//         insert_action(&mut self.versioned.borrow_mut(), id, item::version::Change::create(), create.into());
        
//         Ok(IdT::new(id))
//     }

//     pub fn context(&self) -> &Context {
//         &self.context
//     }

//     // Workaround for player::Manager
//     pub(crate) fn context_mut(&mut self) -> &mut Context {
//         &mut self.context
//     }
// }

// impl<Lookup, Root, Action, Trigger> interaction::Interactor<'_, '_, Lookup, Root, Action, Trigger> {
//     pub fn enqueue_trigger(&mut self, trigger: Trigger) {
//         self.context.enqueue_trigger(trigger);
//     }
// }

// impl<State, Root, Action, Trigger, GameOutcome> reaction::Interactor<'_, '_, State, Root, Action, Trigger, GameOutcome> {
//     pub fn enqueue_trigger(&mut self, trigger: Trigger) {
//         self.context.enqueue_trigger(trigger);
//     }

//     pub fn set_game_outcome(&mut self, game_outcome: GameOutcome) -> Option<GameOutcome> {
//         self.context.set_game_outcome(game_outcome)
//     }
// }

// macro_rules! impl_get_root {
//     ($(<$($ty_param:ident),*> $ty:ty),+) => {
//         $(
//             impl<'l, 'r, Lookup, Action, Root, $($ty_param),*> Interactor<'l, Lookup, Action, $ty> {
//                 pub fn get_root(&mut self) -> Result<Get<'l, '_, Lookup, Action, $ty, Root>, lookup::Error>
//                 where 
//                     Lookup: item::lookup::OfType<Root>,
//                 {
//                     self.get(*self.context().root)
//                 }
//             }
//         )+
//     };
// }

// impl_get_root! {
//     <> player::init::Context<'r, IdT<Root>>,
//     <Trigger> interaction::Context<'r, IdT<Root>, Trigger>,
//     <Trigger, GameOutcome> reaction::Context<'r, IdT<Root>, Trigger, GameOutcome>
// }

// fn insert_action<Action>(versioned: &mut HashMap<item::Id, Versioned<Action>>, id: item::Id, version: item::version::Change, action: Action) {
//     let record = record::Packed::new(id, version, action);

//     use std::collections::hash_map::Entry;
//     match versioned.entry(id) {
//         Entry::Occupied(mut oe) => match oe.get_mut() {
//             Versioned::Read(_) => {
//                 oe.insert(Versioned::Record(record));
//             }
//             Versioned::Record(packed) => packed.append(record.into_action()),
//         },
//         Entry::Vacant(ve) => {
//             ve.insert(Versioned::Record(record));
//         }
//     }
// }

// pub struct Get<'l, 'i, Lookup, Action, Context, T> {
//     interactor: &'i Interactor<'l, Lookup, Action, Context>,
//     item: &'l Item<T>,
// }

// impl<'l, 'i, Lookup, Action, Context, T> Get<'l, 'i, Lookup, Action, Context, T> {
//     fn new(interactor: &'i Interactor<'l, Lookup, Action, Context>, item: &'l Item<T>) -> Self {
//         interactor.versioned.borrow_mut()
//             .entry(item.id().untyped())
//             .or_insert(Versioned::Read(item.version()));

//         Self {
//             interactor,
//             item,
//         }
//     }

//     pub fn get(&self) -> &T {
//         self.item.get()
//     }

//     pub fn update<Update>(&self, update: Update) 
//         -> &Self
//     where 
//         Update:
//             Into<Action> + 
//             action::Update<T = T>,
//     {
//         insert_action(&mut self.interactor.versioned.borrow_mut(), self.item.id().untyped(), item::version::Change::update(self.item.version()), update.into());
//         self
//     }

//     pub fn destroy<Destroy>(&self, destroy: Destroy)
//         -> &Self
//     where 
//         Destroy: 
//             Into<Action> + 
//             action::Destroy<T = T>,
//     {
//         insert_action(&mut self.interactor.versioned.borrow_mut(), self.item.id().untyped(), item::version::Change::destroy(self.item.version()), destroy.into());
//         self
//     }

//     pub fn follow<'u, U>(&'u self, id: IdT<U>) -> Result<Get<'l, 'u, Lookup, Action, Context, U>, lookup::Error> 
//     where 
//         'u: 'l,
//         Lookup: lookup::OfType<U>,
//     {
//         self.interactor.get(id)
//     }
// }

// impl<'l, 'i, Lookup, Action, Context, T> ops::Deref for Get<'l, 'i, Lookup, Action, Context, T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         self.item.get()
//     }
// }

#[derive(Debug)]
enum Versioned<Action> {
    Read(item::Version),
    Record(record::Packed<Action>),
}

impl<Action> Versioned<Action> {
    fn expected(&self) -> item::Version {
        match self {
            Versioned::Read(version) => *version,
            Versioned::Record(packed) => packed.version_change().before,
        }
    }
}