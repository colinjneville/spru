// use std::{collections::HashMap, marker::PhantomData, hash::Hash};

// use spru::Serial;

// use crate::Strictness;

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// #[derive(thiserror::Error)]
// pub enum TryInsertError {
//     #[error("Key already exists")]
//     KeyExists,
// }

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// #[derive(thiserror::Error)]
// pub enum TryRemoveError {
//     #[error("Key does not exist")]
//     KeyDoesNotExist,
// }

// #[derive(Debug)]
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct Map<K: Eq + Hash, V> {
//     map: HashMap<K, V>,
// }

// #[derive(Debug)]
// #[derive(serde::Serialize, serde::Deserialize)]
// #[derive(spru::Action, spru::Modify)]
// #[action(new_error = Error, t = Map<K, V>)]
// pub enum Modify<K: Eq + Hash, V> {
//     TryInsert(TryInsert<K, V>),
//     TryRemove(TryRemove<K, V>),
//     Set(Set<K, V>),
// }

// #[derive(Debug)]
// #[derive(serde::Serialize, serde::Deserialize)]
// #[derive(spru::Action)]
// #[action(undo = TryRemove<K, V>, error = TryInsertError, t = Map<K, V>)]
// pub struct TryInsert<K: Eq + Hash, V> {
//     key: K,
//     value: V,
//     strictness: Strictness,
// }

// impl<K: Eq + Hash, V> TryInsert<K, V> {
//     pub fn new(key: K, value: V, strictness: Strictness) -> Self {
//         Self {
//             key,
//             value,
//             strictness,
//         }
//     }
// }

// impl<K: Serial + Eq + Hash + Clone, V: Serial + Clone> spru::Modify for TryInsert<K, V> {
//     fn modify(&self, value: &mut Self::T) -> Result<Option<Self::UndoAction>, Self::Error> {
//         Ok(match value.map.entry(self.key.clone()) {
//             std::collections::hash_map::Entry::Occupied(_) => match self.strictness {
//                 Strictness::BestEffort => None,
//                 Strictness::AllOrError => return Err(Error::KeyExists),
//             },
//             std::collections::hash_map::Entry::Vacant(e) => {
//                 e.insert(self.value.clone());
//                 Some(TryRemove::new(self.key.clone(), Strictness::AllOrError))
//             },
//         })
//     }
// }

// #[derive(Debug)]
// #[derive(serde::Serialize, serde::Deserialize)]
// #[derive(spru::Action)]
// #[action(t = Map<K, V>)]
// pub struct Set<K: Eq + Hash, V> {
//     key: K,
//     value: Option<V>,
// }

// impl<K: Eq + Hash, V> Set<K, V> {
//     pub fn new(key: K, value: Option<V>) -> Self {
//         Self {
//             key,
//             value,
//         }
//     }
// }

// impl<K: Serial + Eq + Hash + Clone, V: Serial + Clone> spru::Modify for Set<K, V> {
//     fn modify(&self, value: &mut Self::T) -> Result<Option<Self::UndoAction>, Self::Error> {
//         Ok(match self.value.clone() {
//             Some(v) => {
//                 let old_v = value.map.insert(self.key.clone(), v);
//                 Some(Set::new(self.key.clone(), old_v))
//             },
//             None => match value.map.remove_entry(&self.key) {
//                 Some((k, v)) => Some(Set::new(k, Some(v))),
//                 None => None,
//             }
//         })
//     }
// }

// #[derive(Debug)]
// #[derive(serde::Serialize, serde::Deserialize)]
// #[derive(spru::Action)]
// #[action(undo = TryInsert<K, V>, error = TryRemoveError, t = Map<K, V>)]
// pub struct TryRemove<K: Eq + Hash, V> {
//     key: K,
//     strictness: Strictness,
//     _p: PhantomData<fn() -> V>,
// }

// impl<K: Eq + Hash, V> TryRemove<K, V> {
//     pub fn new(key: K, strictness: Strictness) -> Self {
//         Self {
//             key,
//             strictness,
//             _p: PhantomData,
//         }
//     }
// }

// impl<K: Serial + Eq + Hash + Clone, V: Serial> spru::Modify for TryRemove<K, V> {
//     fn modify(&self, value: &mut Self::T) -> Result<Option<Self::UndoAction>, Self::Error> {
//         Ok(match value.map.remove(&self.key) {
//             Some(v) => Some(TryInsert::new(self.key.clone(), v, Strictness::AllOrError)),
//             None => match self.strictness {
//                 Strictness::BestEffort => None,
//                 Strictness::AllOrError => return Err(Error::KeyDoesNotExist),
//             }
//         })
//     }
// }