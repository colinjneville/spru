use std::marker::PhantomData;

use derive_where::derive_where;
use spru::common::error::{AnyError, AnyResult};

#[derive(spru::action::Update)]
#[derive_where(Debug, Clone, Default, Serialize, Deserialize; )]
#[must_use]
pub struct Update<T>(String, PhantomData<T>);

impl<T> spru::action::Update for Update<T> {
    type T = T;
    type Undo = Self;

    #[allow(refining_impl_trait)]
    fn update(&self, _value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        Err(AnyError::from_string(&self.0))
    }
}

pub fn fail<T>(err_msg: String) -> Update<T> {
    Update(err_msg, PhantomData)
}