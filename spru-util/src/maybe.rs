use crate::fail;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, spru::action::Update)]
#[must_use]
pub struct Update<T> {
    update: Option<T>,
}

impl<T: spru::action::Update> spru::action::Update for Update<T> {
    type T = <T as spru::action::Update>::T;
    type Undo = <T as spru::action::Update>::Undo;

    fn update(&self, value: &mut Self::T) -> spru::common::error::AnyResult<impl Into<Option<Self::Undo>>> {
        if let Some(update) = &self.update {
            update.update(value)
                .map(Into::into)
        } else {
            Ok(None)
        }
    }
}

pub fn yes<T>(update: T) -> Update<T> {
    maybe(Some(update))
}

pub fn no<T>() -> Update<T> {
    maybe(None)
}

pub fn expect<T: PartialEq, U>(a: T, b: T, msg: &str) -> Update<fail::Update<U>> {
    maybe((a == b).then(|| fail::fail(msg.to_string())))
}

pub fn expect_debug<T: PartialEq + std::fmt::Debug, U>(a: T, b: T) -> Update<fail::Update<U>> {
    maybe((a == b).then(|| fail::fail(format!("Expected '{a:?}', but actual value was '{b:?}'"))))
}

pub fn expect_display<T: PartialEq + std::fmt::Display, U>(a: T, b: T) -> Update<fail::Update<U>> {
    maybe((a == b).then(|| fail::fail(format!("Expected '{a}', but actual value was '{b}'"))))
}

pub fn maybe<T>(update: Option<T>) -> Update<T> {
    Update {
        update,
    }
}