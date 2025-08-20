mod id;
pub use id::Id;
pub mod init;
pub use init::Init;
pub(crate) mod manager;
pub(crate) use manager::Manager;

// use crate::player;

// use std::ops;

// #[derive(Debug)]
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct Player<Data> {
//     data: Data,
// }

// impl<Data> Player<Data> {
//     fn new(data: Data) -> Self {
//         Self {
//             data,
//         }
//     }

//     pub fn data(&self) -> &Data {
//         &self.data
//     }
// }

// #[derive(Debug)]
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct Players<Data> {
//     players: Vec<Player<Data>>,
// }

// impl<Data> Default for Players<Data> {
//     fn default() -> Self {
//         Self { players: Default::default() }
//     }
// }

// impl<Data> Players<Data> {
//     pub(crate) fn next_player_id(&self) -> player::Id {
//         player::Id(self.players.len())
//     }

//     pub(crate) fn add_player(&mut self, player: Player<Data>) -> player::Id {
//         let index = self.players.len();
//         self.players.push(player);
//         player::Id(index)
//     }
// }

// pub struct Iter<'i> {
//     id: usize,
//     iter: std::slice::Iter<'i, Player>,
// }

// impl<'i> Iter<'i> {
//     fn new(players: &'i [Player]) -> Self {
//         Self {
//             id: 0,
//             iter: players.into_iter(),
//         }
//     }
// }

// impl<'i> Iterator for Iter<'i> {
//     type Item = (Id, &'i Player);

//     fn next(&mut self) -> Option<Self::Item> {
//         match self.iter.next() {
//             Some(p) => {
//                 let item = Some((Id(self.id), p));
//                 self.id += 1;
//                 item
//             },
//             None => None,
//         }
//     }
// }

// impl ops::Index<player::Id> for Players {
//     type Output = Player;

//     fn index(&self, index: player::Id) -> &Self::Output {
//         &self.players[index.get()]
//     }
// }

// impl<'i> IntoIterator for &'i Players {
//     type Item = <Self::IntoIter as Iterator>::Item;
//     type IntoIter = Iter<'i>;

//     fn into_iter(self) -> Self::IntoIter {
//         Iter::new(&*self.players)
//     }
// }