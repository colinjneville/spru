use std::{cell::RefCell, collections::HashMap, ops};

use crate::{action, interaction, item::{self, lookup, IdT}, player, reaction, record::{self, Records}, Item};

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

#[derive(Debug)]
pub struct Interactor<'l, Lookup, Action, Context> {
    lookup: &'l Lookup,
    context: Context,
    reservation: &'l item::id::Reservation,
    versioned: RefCell<HashMap<item::Id, Versioned<Action>>>,
}

impl<'l, Lookup, Action, Context> Interactor<'l, Lookup, Action, Context> {
    pub(crate) fn new(lookup: &'l Lookup, reservation: &'l item::id::Reservation, context: Context) -> Self {
        Self {
            lookup,
            context,
            reservation,
            versioned: RefCell::new(HashMap::new()),
        }
    }

    pub fn lookup(&self) -> &'l Lookup {
        self.lookup
    }

    pub(crate) fn expected_versions(&self) -> item::version::Expected {
        let versioned = self.versioned.borrow();
        let iter = versioned.iter()
            .map(|(&id, versioned)| (id, versioned.expected()));
        item::version::Expected::new(iter)
    }

    pub(crate) fn take_records(&mut self) -> Records<Action> {
        let mut records = Records::new();
        for (_, versioned) in self.versioned.borrow_mut().drain() {
            if let Versioned::Record(packed) = versioned {
                records.push(packed);
            }
        }

        records
    }

    pub(crate) fn into_context(self) -> Context {
        self.context
    }

    pub fn get<'i, T>(&'i self, id: IdT<T>) -> Result<Get<'l, 'i, Lookup, Action, Context, T>, Lookup::Error> 
    where
        Lookup: lookup::OfType<T>,
    {
        let item: &Item<T> = self.lookup.lookup(id)?;

        Ok(Get::new(self, item))
    }

    pub fn create<Create>(&self, create: Create) -> Result<IdT<Create::T>, Lookup::Error> 
    where 
        Lookup: item::Lookup,
        Create: 
            Into<Action> + 
            action::Create,
    {
        let id = self.reservation.claim_id().unwrap_or_else(|| unimplemented!());

        insert_action(&mut self.versioned.borrow_mut(), id, item::version::Change::create(), create.into());
        
        Ok(IdT::new(id))
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    // Workaround for player::Manager
    pub(crate) fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }
}

impl<Lookup, Root, Action, Trigger> interaction::Interactor<'_, '_, Lookup, Root, Action, Trigger> {
    pub fn enqueue_trigger(&mut self, trigger: Trigger) {
        self.context.enqueue_trigger(trigger);
    }
}

impl<State, Root, Action, Trigger, GameOutcome> reaction::Interactor<'_, '_, State, Root, Action, Trigger, GameOutcome> {
    pub fn enqueue_trigger(&mut self, trigger: Trigger) {
        self.context.enqueue_trigger(trigger);
    }

    pub fn set_game_outcome(&mut self, game_outcome: GameOutcome) -> Option<GameOutcome> {
        self.context.set_game_outcome(game_outcome)
    }
}

macro_rules! impl_get_root {
    ($(<$($ty_param:ident),*> $ty:ty),+) => {
        $(
            impl<'l, 'r, Lookup, Action, Root, $($ty_param),*> Interactor<'l, Lookup, Action, $ty> {
                pub fn get_root(&mut self) -> Result<Get<'l, '_, Lookup, Action, $ty, Root>, Lookup::Error>
                where 
                    Lookup: item::lookup::OfType<Root>,
                {
                    self.get(*self.context().root)
                }
            }
        )+
    };
}

impl_get_root! {
    <> player::init::Context<'r, IdT<Root>>,
    <Trigger> interaction::Context<'r, IdT<Root>, Trigger>,
    <Trigger, GameOutcome> reaction::Context<'r, IdT<Root>, Trigger, GameOutcome>
}

fn insert_action<Action>(versioned: &mut HashMap<item::Id, Versioned<Action>>, id: item::Id, version: item::version::Change, action: Action) {
    let record = record::Packed::new(id, version, action);

    use std::collections::hash_map::Entry;
    match versioned.entry(id) {
        Entry::Occupied(mut oe) => match oe.get_mut() {
            Versioned::Read(_) => {
                oe.insert(Versioned::Record(record));
            }
            Versioned::Record(packed) => packed.append(record.into_action()),
        },
        Entry::Vacant(ve) => {
            ve.insert(Versioned::Record(record));
        }
    }
}

pub struct Get<'l, 'i, Lookup, Action, Context, T> {
    interactor: &'i Interactor<'l, Lookup, Action, Context>,
    item: &'i Item<T>,
}

impl<'l, 'i, Lookup, Action, Context, T> Get<'l, 'i, Lookup, Action, Context, T> {
    fn new(interactor: &'i Interactor<'l, Lookup, Action, Context>, item: &'i Item<T>) -> Self {
        interactor.versioned.borrow_mut()
            .entry(item.id().untyped())
            .or_insert(Versioned::Read(item.version()));

        Self {
            interactor,
            item,
        }
    }

    pub fn get(&self) -> &T {
        self.item.get()
    }

    pub fn update<Update>(&self, update: Update) 
        -> &Self
    where 
        Update:
            Into<Action> + 
            action::Update<T = T>,
    {
        insert_action(&mut self.interactor.versioned.borrow_mut(), self.item.id().untyped(), item::version::Change::update(self.item.version()), update.into());
        self
    }

    pub fn destroy<Destroy>(&self, destroy: Destroy)
        -> &Self
    where 
        Destroy: 
            Into<Action> + 
            action::Destroy<T = T>,
    {
        insert_action(&mut self.interactor.versioned.borrow_mut(), self.item.id().untyped(), item::version::Change::destroy(self.item.version()), destroy.into());
        self
    }

    pub fn follow<'u, U>(&'u self, id: IdT<U>) -> Result<Get<'l, 'u, Lookup, Action, Context, U>, Lookup::Error> 
    where 
        'u: 'l,
        Lookup: lookup::OfTypeMut<U>,
    {
        self.interactor.get(id)
    }
}

impl<'l, 'i, Lookup, Action, Context, T> ops::Deref for Get<'l, 'i, Lookup, Action, Context, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.item.get()
    }
}

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