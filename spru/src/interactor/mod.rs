use std::{cell::RefCell, collections::{HashMap, VecDeque}, mem, ops};

use crate::{action, error::{RecoverableError, RecoverableResult}, interactor, item::{self, lookup, IdT}, player, record::{self, Records}, Item};

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
                let $get = get;
                let next_id = $next_id;

                let get = match $get.follow(next_id) {
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

    fn version_change(&self) -> item::version::Change {
        // We only need to bump the version number for the first (non-noop) change, 
        // but we need to keep track of the first change to publish expected versions
        if self.flushed_do.is_empty() {
            self.version_change
        } else {
            self.version_change.into_noop()
        }
    }

    fn update_immediate<'l, Lookup, Update>(&mut self, id: item::Id, lookup: &'l mut Lookup, update: Update)
        -> action::Result<&'l Update::T>
    where
        Lookup:
            item::Lookup,
        Action: 
            crate::Action<State = Lookup::State>,
        Update:
            Into<Action> +
            action::Update<T: item::lookup::Lookupable<Lookup::State>, Undo: Into<Action>>,
    {
        self.flush(id, lookup)?;

        let context = action::Context::new(lookup, id, self.version_change());
        let undo = context.update(&update)?;
        if let Some(undo) = undo {
            self.flushed_do.push(update.into());
            self.flushed_undo.push(undo.into());
        }

        let item = lookup.lookup::<Update::T>(id.force_type())
            .expect("Item must still exist");
        Ok(item.get())
    }

    fn flush<Lookup>(&mut self, id: item::Id, lookup: &mut Lookup) 
        -> action::Result<()>
    where 
        Action: crate::Action<State = Lookup::State>,
        Lookup: item::Lookup,
    {
        for pending_do in mem::take(self.pending_do.get_mut()) {
            let context = action::Context::new(lookup, id, self.version_change());
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
        Action: crate::Action<State = Lookup::State>,
        Lookup: item::Lookup,
    {
        // Only undo the version with the first change if there are multiple
        let mut version_change = self.version_change.undo();
        for undo in self.flushed_undo.into_iter().rev() {
            let context = action::Context::new(lookup, id, version_change);
            let _redo = undo.apply(context)?;

            version_change = version_change.into_noop();
        }

        Ok(())
    }

    fn expected_version(&self) -> item::Version {
        self.version_change.before
    }

    fn into_records(self, id: item::Id) -> Option<(record::Packed<Action>, record::Packed<Action>)> {
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

    fn update_immediate<'l, Lookup, Update>(&mut self, id: item::Id, lookup: &'l mut Lookup, update: Update)
        -> action::Result<&'l Update::T>
    where
        Lookup:
            item::Lookup,
        Action: 
            crate::Action<State = Lookup::State>,
        Update:
            Into<Action> +
            action::Update<T: item::lookup::Lookupable<Lookup::State>, Undo: Into<Action>>,
    {
        self.items.borrow_mut()
            .get_mut(&id)
            .expect("id must be added as read first")
            .update_immediate(id, lookup, update)
    }

    fn flush<Lookup>(&mut self, lookup: &mut Lookup)
        -> action::Result<()>
    where 
        Action: crate::Action<State = Lookup::State>,
        Lookup: item::Lookup,
    {
        for (&id, item) in self.items.get_mut() {
            item.flush(id, lookup)?;
        }
        
        Ok(())
    }

    fn is_item_flushed(&self, id: item::Id) -> bool {
        let items = self.items.borrow();
        if let Some(item) = items.get(&id) {
            item.is_flushed()
        } else {
            true
        }
    }

    fn revert<Lookup>(self, lookup: &mut Lookup)
        -> action::Result<()>
    where 
        Action: crate::Action<State = Lookup::State>,
        Lookup: item::Lookup,
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

    fn into_records(self) -> (Records<Action>, Records<Action>) {
        let mut packed_do = Records::new();
        let mut packed_undo = Records::new();
        for (item_id, item) in self.items.into_inner() {
            if let Some((item_do, item_undo)) = item.into_records(item_id) {
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

impl<'l, Lookup, Action> Inner<'l, Lookup, Action> {
    fn get<T>(&self, id: IdT<T>) 
        -> lookup::Result<Existing<'_, Lookup, Action, T>>
    where
        Lookup: item::Lookup,
        T: item::lookup::Lookupable<Lookup::State>
    {
        let item = self.lookup.lookup(id)
            .map_err(|e| e.with_context(id))?;
        self.items_status.register_read(id.untyped(), item.version());

        Ok(Existing {
            inner: self,
            item,
        })
    }
}

#[derive(Debug)]
pub struct Interactor<'l, Lookup, Action, Context, Output> {
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
        -> Pending<'_, Lookup, Action, Create::T>
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
        -> lookup::Result<Existing<'_, Lookup, Action, T>>
    where
        Lookup: item::Lookup,
        T: item::lookup::Lookupable<Lookup::State>
    {
        self.inner.get(id)
    }

    pub fn get_root<Root>(&self)
        -> lookup::Result<Existing<'_, Lookup, Action, Root>>
    where
        Lookup: item::Lookup,
        Root: item::lookup::Lookupable<Lookup::State>,
        Context: GetRoot<Root=IdT<Root>>,
    {
        let root_id = *self.context.get_root();
        self.get(root_id)
    }

    pub fn update_immediate<Update>(&mut self, update: WithUpdate<Update::T, Update>)
        -> action::Result<&Update::T>
    where
        Lookup:
            item::Lookup,
        Action: 
            crate::Action<State = Lookup::State>,
        Update:
            Into<Action> +
            action::Update<T: item::lookup::Lookupable<Lookup::State>, Undo: Into<Action>>,
    {
        let WithUpdate {
            id,
            update,
        } = update;

        self.flush()?;
        self.inner.items_status.update_immediate(id.untyped(), self.inner.lookup, update)
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
        -> action::Result<()>
    where 
        Action: crate::Action<State = Lookup::State>,
        Lookup: item::Lookup,
    {
        self.inner.items_status.flush(self.inner.lookup)
    }

    pub(crate) fn revert<E>(self, err: E)
        -> RecoverableError<E>
    where 
        Action: crate::Action<State = Lookup::State>,
        Lookup: item::Lookup,
    {
        let mut recoverable_error = RecoverableError::new(err);
        if let Err(recovery_err) = self.inner.items_status.revert(self.inner.lookup) {
            recoverable_error.set_recovery_error(recovery_err);
        }

        recoverable_error
    }

    pub(crate) fn complete<E>(mut self, error: Option<E>) 
        -> RecoverableResult<interactor::Complete<Action, Context, Output>, E> 
    where 
        Action: crate::Action<State = Lookup::State>,
        action::Error: Into<E>,
        Lookup: item::Lookup,
    {
        let result = match error {
            None => self.flush()
                .map_err(Into::into),
            Some(err) => Err(err),
        };

        match result {
            Ok(_) => Ok(self.complete_internal()),
            Err(err) => Err(self.revert(err)),
        }
    }

    // Interactor must be flushed before calling `complete`
    fn complete_internal(self) -> Complete<Action, Context, Output> {
        let Self {
            inner: Inner {
                lookup: _lookup,
                items_status,
                reservation: _reservation,
            },
            context,
            output,
        } = self;

        let expected_versions = items_status.expected_versions();
        let (do_records, undo_records) = items_status.into_records();

        Complete {
            expected_versions,
            do_records,
            undo_records,
            context,
            output: output.into_inner(),
        }
    }
}

pub(crate) struct Complete<Action, Context, Output> {
    pub(crate) expected_versions: item::version::Expected,
    pub(crate) do_records: Records<Action>,
    pub(crate) undo_records: Records<Action>,
    pub(crate) context: Context,
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

pub(crate) trait TakeTriggers<Trigger> {
    fn take_triggers(&mut self) -> VecDeque<Trigger>;
}

pub(crate) trait TakeGameOutcome<GameOutcome> {
    fn take_game_outcome(&mut self) -> Option<GameOutcome>;
}

pub(crate) trait PlayerContext {
    fn player_context(&self) -> Option<player::Id>;
}

#[doc(hidden)]
pub trait GetRoot {
    type Root;

    fn get_root(&self) -> &Self::Root;
}

#[derive(Debug)]
pub struct Pending<'i, Lookup, Action, T> {
    inner: &'i Inner<'i, Lookup, Action>,
    item_id: IdT<T>,
}

impl<'i, Lookup, Action, T> Pending<'i, Lookup, Action, T> {
    pub fn id(&self) -> IdT<T> {
        self.item_id
    }

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
    pub fn id(&self) -> IdT<T> {
        self.item.id()
    }

    pub fn follow<U>(&self, id: IdT<U>)
        -> lookup::Result<Existing<'i, Lookup, Action, U>>
    where
        Lookup: item::Lookup,
        U: item::lookup::Lookupable<Lookup::State>
    {
        self.inner.get(id)
    }

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

    
    pub fn update_immediate<Update>(self, update: Update)
        -> WithUpdate<T, Update>
    {
        WithUpdate {
            id: self.id(),
            update,
        }
    }

    pub fn destroy<Destroy>(self, destroy: Destroy) 
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

#[must_use]
#[derive(Debug)]
pub struct WithUpdate<T, Update> {
    id: IdT<T>,
    update: Update,
}

