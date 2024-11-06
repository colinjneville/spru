use crate::action::verbatim;

pub struct Bag<T> {
    items: Vec<T>,
}

impl<T> Bag<T> {
    // pub fn peek_next(&self, seed: u64) -> Option<&T> {
    //     self.index_from_seed(seed).map(|i| &self.items[i])
    // }

    // fn index_from_seed(&self, seed: u64) -> Option<usize> {
    //     if self.items.is_empty() {
    //         None
    //     } else {
    //         use rand::{Rng, SeedableRng};

    //         let mut rng = crate::Rng::seed_from_u64(seed);
    //         Some(rng.gen_range(0..self.items.len()))
    //     }
    // }
}

pub type Create<T> = verbatim::Create<Bag<T>>;

// pub enum Modify<T> {
//     Remove
// }

pub type Destroy<T> = verbatim::Destroy<Bag<T>>;