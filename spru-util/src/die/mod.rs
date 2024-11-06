mod uniform_die;
pub use uniform_die::UniformDie;

use rand::Rng;

pub trait Die<Output> {
    fn roll<R: Rng>(&self, rng: &mut R) -> Output;
}

//pub trait NumericDie<Output: 