use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    mem, ops,
};

use crate::{
    Item, action,
    common::error::RecoverableError,
    interactor,
    item::{self, IdT, storage},
    player,
    record::{self, Records},
};

use derive_where::derive_where;

/// A macro to more conveniently convert [`IdT<T>`]s to `&T`.  
/// ```ignore
/// with! { <interactor-ident> =>
///     <statements>
/// };
/// ```
/// Within `<statements>`, you can use `~[<idt-expression>]` as an
/// operator to convert an [IdT] to a [`storage::Result<Existing>`]:
/// ```rust,ignore
/// with! { interactor =>
///     let root = interactor.get_root()?;
///     let deck = ~[root.deck]?;
///     let player0 = ~[~[root.players]?.first()]?
/// };
/// ```
/// Note: this macro is likely to change in the future to improve
/// ergonomics, LSP-friendliness, and versitility.
#[doc(inline)]
pub use spru_macro::with;

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
        self.pending_do.borrow_mut().push_back(action);
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

    // fn create_immediate<'l, Storage, Create>(
    //     &mut self,
    //     id: item::Id,
    //     storage: &'l mut Storage,
    //     create: Create,
    // ) -> action::Result<Self>
    // where
    //     Storage: item::Storage,
    //     Action: crate::Action<State = Storage::State>,
    //     Create: Into<Action> + action::Create<T: item::storage::Storable<Storage::State>, Undo: Into<Action>>,
    // {
    //     let mut this = Self::pending();
        
    //     let context = action::Context::new(storage, id, this.version_change());
    //     let undo = context.create(&create)?;
    //     if let Some(undo) = undo {
    //         this.flushed_do.push(create.into());
    //         this.flushed_undo.push(undo.into());
    //     }

    //     Ok(this)
    // }

    // fn update_immediate<'l, Storage, Update>(
    //     &mut self,
    //     id: item::Id,
    //     storage: &'l mut Storage,
    //     update: Update,
    // ) -> action::Result<&'l Update::T>
    // where
    //     Storage: item::Storage,
    //     Action: crate::Action<State = Storage::State>,
    //     Update: Into<Action>
    //         + action::Update<T: item::storage::Storable<Storage::State>, Undo: Into<Action>>,
    // {
    //     self.flush(id, storage)?;

    //     let context = action::Context::new(storage, id, self.version_change());
    //     let undo = context.update(&update)?;
    //     if let Some(undo) = undo {
    //         self.flushed_do.push(update.into());
    //         self.flushed_undo.push(undo.into());
    //     }

    //     let item = storage
    //         .get::<Update::T>(id.force_type())
    //         .expect("Item must still exist");
    //     Ok(item.get())
    // }

    // fn destroy_immediate<'l, Storage, Destroy>(
    //     &mut self,
    //     id: item::Id,
    //     storage: &'l mut Storage,
    //     destroy: Destroy,
    // ) -> action::Result<()>
    // where
    //     Storage: item::Storage,
    //     Action: crate::Action<State = Storage::State>,
    //     Destroy: Into<Action> + action::Destroy<T: item::storage::Storable<Storage::State>, Undo: Into<Action>>,
    // {
    //     self.flush(id, storage)?;

    //     let context = action::Context::new(storage, id, self.version_change());
    //     let undo = context.destroy(&destroy)?;
    //     if let Some(undo) = undo {
    //         self.flushed_do.push(destroy.into());
    //         self.flushed_undo.push(undo.into());
    //     }

    //     Ok(())
    // }

    fn flush<Storage>(&mut self, id: item::Id, storage: &mut Storage) -> action::Result<()>
    where
        Action: crate::Action<State = Storage::State>,
        Storage: item::Storage,
    {
        for pending_do in mem::take(self.pending_do.get_mut()) {
            let context = action::Context::new(storage, id, self.version_change());
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

    fn revert<Storage>(self, id: item::Id, storage: &mut Storage) -> action::Result<()>
    where
        Action: crate::Action<State = Storage::State>,
        Storage: item::Storage,
    {
        // Only undo the version with the first change if there are multiple
        let mut version_change = self.version_change.undo();
        for undo in self.flushed_undo.into_iter().rev() {
            let context = action::Context::new(storage, id, version_change);
            let _redo = undo.apply(context)?;

            version_change = version_change.into_noop();
        }

        Ok(())
    }

    fn expected_version(&self) -> item::Version {
        self.version_change.before
    }

    fn into_records(
        self,
        id: item::Id,
    ) -> Option<(record::Packed<Action>, record::Packed<Action>)> {
        if !self.is_flushed() {
            panic!("Interactor must be flushed first");
        }

        let mut flushed_do = self.flushed_do.into_iter();
        if let Some(first_do) = flushed_do.next() {
            let mut packed_do = record::Packed::new(id, self.version_change, first_do);
            for do_action in flushed_do {
                packed_do.append(do_action);
            }

            let mut flushed_undo = self.flushed_undo.into_iter();
            let first_undo = flushed_undo
                .next()
                .expect("do and undo must have same length");
            let mut packed_undo = record::Packed::new(id, self.version_change.undo(), first_undo);
            for undo_action in flushed_undo {
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
        self.items
            .borrow_mut()
            .entry(id)
            .or_insert(ItemStatus::existing(version));
    }

    fn enqueue_create(&self, id: item::Id, action: Action) {
        self.items
            .borrow_mut()
            .entry(id)
            .or_insert(ItemStatus::pending())
            .enqueue(action);
    }

    fn enqueue(&self, id: item::Id, action: Action) {
        self.items
            .borrow_mut()
            .get_mut(&id)
            .expect("id must be added as read first")
            .enqueue(action);
    }

    // fn create_immediate<'l, Storage, Create>(
    //     &mut self,
    //     id: item::Id,
    //     storage: &'l mut Storage,
    //     create: Create,
    // ) -> action::Result<&'l Create::T>
    // where
    //     Storage: item::Storage,
    //     Action: crate::Action<State = Storage::State>,
    //     Create: Into<Action> + action::Create<T: item::storage::Storable<Storage::State>, Undo: Into<Action>>,
    // {
    //     self.items
    //         .borrow_mut()
    //         .entry(id)
    //         .or_insert_with(|| ItemStatus::pending())
    //         .create_immediate(id, storage, create)
    // }

    // fn update_immediate<'l, Storage, Update>(
    //     &mut self,
    //     id: item::Id,
    //     storage: &'l mut Storage,
    //     update: Update,
    // ) -> action::Result<&'l Update::T>
    // where
    //     Storage: item::Storage,
    //     Action: crate::Action<State = Storage::State>,
    //     Update: Into<Action>
    //         + action::Update<T: item::storage::Storable<Storage::State>, Undo: Into<Action>>,
    // {
    //     self.items
    //         .borrow_mut()
    //         .get_mut(&id)
    //         .expect("id must be added as read first")
    //         .update_immediate(id, storage, update)
    // }

    fn flush<Storage>(&mut self, storage: &mut Storage) -> action::Result<()>
    where
        Action: crate::Action<State = Storage::State>,
        Storage: item::Storage,
    {
        for (&id, item) in self.items.get_mut() {
            item.flush(id, storage)?;
        }

        Ok(())
    }

    fn revert<Storage>(self, storage: &mut Storage) -> action::Result<()>
    where
        Action: crate::Action<State = Storage::State>,
        Storage: item::Storage,
    {
        for (id, item) in self.items.into_inner() {
            item.revert(id, storage)?;
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

/// A subcomponent of [Interactor] responsible for recording [Item] reads and writes.
/// Normally, you do not need to use this directly, but in advanced scenarios, this 
/// provides a common interface between all [Interactor] types in a single game (i.e.
/// no Context or Output type parameters).
#[derive(Debug)]
pub struct Ledger<'l, Storage, Action> {
    storage: &'l mut Storage,
    items_status: ItemsStatus<Action>,
    reservation: &'l item::id::Reservation,
}

impl<'l, Storage, Action> Ledger<'l, Storage, Action> {
    /// See [Interactor::get].
    pub fn get<T>(&self, id: IdT<T>) -> storage::Result<Existing<'_, Storage, Action, T>>
    where
        Storage: item::Storage,
        T: item::storage::Storable<Storage::State>,
    {
        let item = self.storage.get(id).map_err(|e| e.with_context(id))?;
        self.items_status
            .register_read(id.untyped(), item.version());

        Ok(Existing { inner: self, item })
    }
}


#[cfg(feature = "test-util")]
pub mod test_util {
    use super::*;

    #[derive(Debug)]
    pub struct TestInteractor<Storage> {
        storage: Storage,
        reservation: item::id::Reservation,
    }

    impl<Storage> TestInteractor<Storage> {
        pub fn new(storage: Storage) -> Self {
            Self {
                storage,
                reservation: item::id::Reservation::all(),
            }
        }

        pub fn interactor<Action, Root>(&mut self, root: Root) -> Interactor<'_, Storage, Action, TestContext<Root>, ()> {
            let ledger = Ledger {
                storage: &mut self.storage,
                items_status: Default::default(),
                reservation: &self.reservation,
            };
            Interactor { ledger, context: TestContext { root }, output: RefCell::new(()) }
        }
    }

    #[derive(Debug)]
    pub struct TestContext<Root> {
        root: Root,
    }

    impl<Root> GetRoot for TestContext<Root> {
        type Root = Root;
    
        fn get_root(&self) -> &Self::Root {
            &self.root
        }
    }
}


/// The interface all modifications of the game state go through.  
///
/// Game state modification is deferred by default. Each change in queued until the end of the
/// function, or [Interactor::flush] is called. This means all reads will not see any changes made
/// after the last flush, if any. Multiple changes can be queued to the same item between flush points,
/// and they will be applied in the order they were queued.  
///
/// Use [Interactor::create] to create new items, and [Interactor::get] to storage existing items. You
/// can queue updates in either case using [Pending::update] or [Existing::update], respectively. With
/// an [Existing] item, you can read the state of the item by dereferencing it, and use [Existing::destroy]
/// to destroy it.  
///
/// Interactors are used in [game::Init](crate::game::Init), [player::Init],
/// [Interaction](trait@crate::Interaction), and [Reaction](trait@crate::Reaction). Depending on the usage,
/// an Interactor will have different `Context` available. For example, in
/// most cases, the `Context` will contain the game [Root](crate::Common::Root), but in game::Init,
/// it does not, as the Root has not yet been created.  
///
/// Interactors may also have additional optional outputs, depending on the kind. Interactions and
/// Reactions may use [Interactor::enqueue_trigger] to enqueue [Reaction::Trigger](crate::Reaction::Trigger)s,
/// which will start a new Reaction on the server once dequeued. Reactions can also end the game by
/// [Interactor::set_game_outcome].  
///
/// All changes made with an Interactor are transactional: if an error occurs, there is no need to undo
/// any changes you made (even if flushed), just return an error. The changes from that Interactor and
/// all the preceding triggering Interactors will be reverted automatically.
#[derive(Debug)]
pub struct Interactor<'l, Storage, Action, Context, Output> {
    ledger: Ledger<'l, Storage, Action>,
    context: Context,
    output: RefCell<Output>,
}

impl<'l, Storage, Action, Context, Output> Interactor<'l, Storage, Action, Context, Output> {
    pub(crate) fn new(
        storage: &'l mut Storage,
        reservation: &'l item::id::Reservation,
        context: Context,
    ) -> Self
    where
        Output: Default,
    {
        Self {
            ledger: Ledger {
                storage,
                items_status: ItemsStatus::default(),
                reservation,
            },
            context,
            output: RefCell::new(Output::default()),
        }
    }

    /// Direct access to the [Ledger]. Only needed in rare cases to minimize type parameters.
    pub fn ledger(&self) -> &Ledger<'l, Storage, Action> {
        &self.ledger
    }

    /// Extra context available to this Interactor
    pub fn context(&self) -> &Context {
        &self.context
    }

    // Hack for player init that should be refactored at some point
    pub(crate) fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Create a new item. See [action::Create].  
    ///
    /// As changes are deferred, this method returns a [Pending] item, which can make updates,
    /// but cannot read the item state, as it does not exist until [Interactor::flush] is called.  
    pub fn create<Create>(&self, create: Create) -> Pending<'_, Storage, Action, Create::T>
    where
        Create: Into<Action> + action::Create,
    {
        let item_id = self
            .ledger
            .reservation
            .claim_id()
            .unwrap_or_else(|| unimplemented!("Out of ids"));
        self.ledger
            .items_status
            .enqueue_create(item_id, create.into());

        Pending {
            inner: &self.ledger,
            item_id: item_id.force_type(),
        }
    }

    // TODO This is needed to simplify scripting bindings, but shouldn't be used manually.
    #[doc(hidden)]
    pub fn enqueue_action(&self, id: item::Id, action: Action) {
        self.ledger
        // `enqueue_create` is used to allow both Create and non-Create Actions
            .items_status.enqueue_create(id, action);
    }

    /// Find an existing item by id. Returns an error if the item does not exist.  
    pub fn get<T>(&self, id: IdT<T>) -> storage::Result<Existing<'_, Storage, Action, T>>
    where
        Storage: item::Storage,
        T: item::storage::Storable<Storage::State>,
    {
        self.ledger.get(id)
    }

    /// Gets the [Common::Root](crate::Common::Root) object.
    pub fn root(&self) -> &Context::Root
    where
        Context: GetRoot,
    {
        self.context.get_root()
    }

    /// If the [Common::Root](crate::Common::Root) object is an [IdT], gets that item.  
    /// Equivalent to `self.get(self.root())`.
    pub fn get_root<Root>(&self) -> storage::Result<Existing<'_, Storage, Action, Root>>
    where
        Storage: item::Storage,
        Root: item::storage::Storable<Storage::State>,
        Context: GetRoot<Root = IdT<Root>>,
    {
        let root_id = *self.context.get_root();
        self.get(root_id)
    }

    /// Enqueues a [Reaction](trait@crate::Reaction) with the given [Reaction::Trigger](crate::Reaction::Trigger).  
    ///
    /// Triggers are processed FIFO, after an [Interaction](trait@crate::Interaction) or [Reaction](trait@crate::Reaction) completes,
    /// until the queue is emptied. This means infinite loops are possible if Reactions keep enqueuing more Reactions.  
    ///
    /// Triggers are only executed server-side, and are ignored on the client. If the Reactions are successful, the
    /// clients will be updated with the outcomes of the Reactions. If a Reaction errors, all Reactions and the
    /// initial Interaction (if any) will be reverted, and the resulting game state will be as if they never happened.
    pub fn enqueue_trigger(&self, trigger: Output::Trigger)
    where
        Output: EnqueueTrigger,
    {
        self.output.borrow_mut().enqueue_trigger(trigger);
    }

    /// Sets the game outcome. Once [Reaction](trait@crate::Reaction)s complete successfully, the game will end,
    /// and clients will be notified of the outcome. If this is called multiple times, the last outcome set will be used.
    /// If the transaction fails, the game will not end.
    pub fn set_game_outcome(&self, game_outcome: Output::GameOutcome)
    where
        Output: SetGameOutcome,
    {
        self.output.borrow_mut().set_game_outcome(game_outcome);
    }

    /// Flushes all pending changes to the game state.  
    ///
    /// Avoid calling this method unless necessary, as it requires exclusive access to the Interactor
    /// and is only needed when the updated state of an item needs to be read. It is not necessary to
    /// flush at the end of the function, this will be done automatically.  
    ///
    /// Flush will direct all operations to the [item::Storage] implementation, which means this may
    /// have side effects such as engine events (e.g. entity creation) being triggered. As the
    /// Interactor could still fail, this could result in spurious events as the changes are done
    /// and undone.
    pub fn flush(&mut self) -> action::Result<()>
    where
        Action: crate::Action<State = Storage::State>,
        Storage: item::Storage,
    {
        self.ledger.items_status.flush(self.ledger.storage)
    }

    pub(crate) fn revert<E>(self, err: E) -> RecoverableError<E>
    where
        Action: crate::Action<State = Storage::State>,
        Storage: item::Storage,
    {
        let mut recoverable_error = RecoverableError::new(err);
        if let Err(recovery_err) = self.ledger.items_status.revert(self.ledger.storage) {
            recoverable_error.set_recovery_error(recovery_err);
        }

        recoverable_error
    }

    pub(crate) fn complete<E>(
        mut self,
        error: Option<E>,
    ) -> Result<interactor::Complete<Action, Context, Output>, RecoverableError<E>>
    where
        Action: crate::Action<State = Storage::State>,
        action::Error: Into<E>,
        Storage: item::Storage,
    {
        let result = match error {
            None => self.flush().map_err(Into::into),
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
            ledger:
                Ledger {
                    storage: _st,
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

/// An item queued for creation.  
///
/// You can schedule additional updates, but the item does not exist to read until
/// [Interactor::flush] is called.
#[derive(Debug)]
pub struct Pending<'i, Storage, Action, T> {
    inner: &'i Ledger<'i, Storage, Action>,
    item_id: IdT<T>,
}

impl<'i, Storage, Action, T> Pending<'i, Storage, Action, T> {
    /// The id of the item to be created
    pub fn id(&self) -> IdT<T> {
        self.item_id
    }

    /// Schedule an update to the item
    pub fn update<Update>(&self, update: Update) -> &Self
    where
        Update: Into<Action> + action::Update<T = T>,
    {
        self.inner
            .items_status
            .enqueue(self.item_id.untyped(), update.into());
        self
    }
}

/// An existing item.  
#[derive(Debug)]
pub struct Existing<'i, Storage, Action, T> {
    inner: &'i Ledger<'i, Storage, Action>,
    item: &'i Item<T>,
}

impl<'i, Storage, Action, T> Existing<'i, Storage, Action, T> {
    /// The id of the item
    pub fn id(&self) -> IdT<T> {
        self.item.id()
    }

    /// Update the item. See [action::Update].
    pub fn update<Update>(&self, update: Update) -> &Self
    where
        Update: Into<Action> + action::Update<T = T>,
    {
        self.inner
            .items_status
            .enqueue(self.item.id().untyped(), update.into());
        self
    }

    // // TODO this isn't well-tested yet
    // #[doc(hidden)]
    // pub fn update_immediate<Update>(self, update: Update) -> WithUpdate<T, Update> {
    //     WithUpdate {
    //         id: self.id(),
    //         update,
    //     }
    // }

    /// Destroy the item. See [action::Destroy].
    pub fn destroy<Destroy>(self, destroy: Destroy)
    where
        Destroy: Into<Action> + action::Destroy<T = T>,
    {
        self.inner
            .items_status
            .enqueue(self.item.id().untyped(), destroy.into());
    }
}

impl<'i, Storage, Action, T> ops::Deref for Existing<'i, Storage, Action, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.item.get()
    }
}

// // TODO this isn't well-tested yet
// #[doc(hidden)]
// #[must_use]
// #[derive(Debug)]
// pub struct WithUpdate<T, Update> {
//     id: IdT<T>,
//     update: Update,
// }
