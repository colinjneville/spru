use std::ops;

use spru_script::script;

use crate::cloned;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[script(include = [Impl])]
#[repr(transparent)]
pub struct StateCell<T>(pub T);

impl<T> ops::Deref for StateCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[script(partial = Impl)]
impl<T> StateCell<T> 
where
    T: Clone + 'static,
{
    #[create]
    fn create(value: T) -> cloned::Create<StateCell<T>> {
        cloned::create(StateCell(value))
    }

    #[method]
    fn destroy(&self) -> ((), cloned::Destroy<StateCell<T>>) {
        ((), cloned::destroy())
    }

    #[get]
    fn value(&self) -> T {
        self.0.clone()
    }

    #[set(name = value)]
    fn value_set(&self, value: T) -> (cloned::Update<StateCell<T>>, ) {
        (cloned::update(StateCell(value)), )
    }
}

// TODO: This needs to forward telety info
// I don't remember I implemented that...
// pub type Action<T> = cloned::Actions<StateCell<T>>;

pub type Create<T> = cloned::Create<StateCell<T>>;
pub type Update<T> = cloned::Update<StateCell<T>>;
pub type Destroy<T> = cloned::Destroy<StateCell<T>>;

pub fn create<T>(value: T) -> Create<T> {
    cloned::create(StateCell(value))
}

pub fn update<T>(value: T) -> Update<T> {
    cloned::update(StateCell(value))
}

pub fn destroy<T>() -> Destroy<T> {
    cloned::destroy()
}

