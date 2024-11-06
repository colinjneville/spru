pub mod reaction;
pub use reaction::Reaction;

use std::collections::HashMap;


use crate::{action::{self, adapter}, item::{self, lookup, IdT}, player::{self}, record::{self, Records}, Item, Transaction};

#[derive(Debug)]
enum Versioned<ActionCatalog> {
    Read(item::Version),
    Record(record::Packed<ActionCatalog>),
}

impl<ActionCatalog> Versioned<ActionCatalog> {
    fn expected(&self) -> item::Version {
        match self {
            Versioned::Read(version) => *version,
            Versioned::Record(packed) => packed.version_change().before,
        }
    }
}

#[derive(Debug)]
pub struct Interactor<'l, Lookup, ActionCatalog, Root> {
    lookup: &'l Lookup,
    root: IdT<Root>,
    reservation: &'l item::id::Reservation,
    versioned: HashMap<item::Id, Versioned<ActionCatalog>>,
}

impl<'l, Lookup, ActionCatalog, Root> Interactor<'l, Lookup, ActionCatalog, Root>
where 
    Lookup: item::Lookup,
    ActionCatalog: action::Catalog<Lookup>,
{
    pub(crate) fn new(lookup: &'l Lookup, reservation: &'l item::id::Reservation, root: IdT<Root>) -> Self {
        Self {
            lookup,
            reservation,
            root,
            versioned: HashMap::new(),
        }
    }

    // TODO try to prevent accidental Interactor drop

    pub fn root(&self) -> Result<&Item<Root>, Lookup::Error>
    where 
        Lookup: item::lookup::OfType<Root>,
    {
        self.lookup.lookup(&self.root)
    }

    pub fn root_id(&self) -> IdT<Root> {
        self.root
    }

    pub fn lookup(&self) -> &'l Lookup {
        self.lookup
    }

    pub(crate) fn expected_versions(&self) -> item::version::Expected {
        let iter = self.versioned.iter()
            .map(|(&id, versioned)| (id, versioned.expected()));
        item::version::Expected::new(iter)
    }

    pub(crate) fn into_transaction(self) -> Transaction<ActionCatalog> {
        let mut records = Records::new();
        for (_, versioned) in self.versioned {
            if let Versioned::Record(packed) = versioned {
                records.push(packed);
            }
        }

        Transaction::new(records)
    }

    // Perform a version-checked read. If this item has changed on the server,
    // the transaction will be rejected
    pub fn read<T>(&mut self, id: &IdT<T>) -> Result<&T, Lookup::Error> 
    where 
        Lookup: lookup::OfTypeMut<T>,
    {
        let t: &Item<T> = self.lookup.lookup(id)?;
        let entry = self.versioned.entry(*id.untyped());
        entry.or_insert(Versioned::Read(t.version()));
        
        Ok(t.get())
    }

    fn insert_action(&mut self, id: item::Id, version: item::version::Change, action: ActionCatalog) {
        let record = record::Packed::new(id, version, action);

        use std::collections::hash_map::Entry;
        match self.versioned.entry(id) {
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

    pub fn create<T, Action>(&mut self, create: Action) -> Result<IdT<T>, Lookup::Error> 
    where 
        Lookup: lookup::OfTypeMut<T>,
        Action: 
            Into<ActionCatalog> + 
            crate::Action<Adapter = adapter::Create, Error: Into<ActionCatalog::Error>, Undo: Into<ActionCatalog>>
    {
        let id = self.reservation.claim_id().unwrap_or_else(|()| unimplemented!());

        self.insert_action(id, item::version::Change::create(), create.into());
        
        Ok(IdT::new(id))
    }

    pub fn update<T, Action>(&mut self, update: Action, id: &IdT<T>) -> Result<(), Lookup::Error> 
    where 
        Lookup: lookup::OfTypeMut<T>, 
        Action:
            Into<ActionCatalog> + 
            crate::Action<Adapter = adapter::Update, Error: Into<ActionCatalog::Error>, Undo: Into<ActionCatalog>>
    {
        let id = id.into();
        let version = self.lookup.lookup(id)?.version();
        self.insert_action(*id.untyped(), item::version::Change::update(version), update.into());
        
        Ok(())
    }

    pub fn destroy<T, Action>(&mut self, destroy: Action, id: &IdT<T>) -> Result<(), Lookup::Error> 
    where 
        Lookup: lookup::OfTypeMut<T>,
        Action: 
            Into<ActionCatalog> + 
            crate::Action<Adapter = adapter::Destroy, Error: Into<ActionCatalog::Error>, Undo: Into<ActionCatalog>>
    {
        let id = id.into();
        let version = self.lookup.lookup(id)?.version();
        self.insert_action(*id.untyped(), item::version::Change::destroy(version), destroy.into());

        Ok(())
    }
}

pub trait Interaction<ActionCatalog, Root> {
    type Output;
    type Error;
    fn apply<Lookup>(&self, interactor: &mut Interactor<Lookup, ActionCatalog, Root>, player_id: player::Id) -> Result<Self::Output, Error<Lookup::Error, Self::Error>>
    where 
        Lookup: item::Lookup,
        ActionCatalog: action::Catalog<Lookup>,
    ;
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<LookupError, InteractionError> {
    Lookup(LookupError),
    Interaction(InteractionError),
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Staged<Interaction> {
    pub(crate) interaction: Interaction,
    pub(crate) expected_versions: item::version::Expected,
}