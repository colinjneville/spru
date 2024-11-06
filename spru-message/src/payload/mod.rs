pub mod slot;
pub use slot::Slot;
pub mod variant;
pub use variant::Variant;

use std::{any, marker::PhantomData, mem};

use crate::SerializeError;

mod private {
    pub trait Sealed { }
}

pub trait PayloadSize: private::Sealed {
    type Payload;

    fn byte_len(&self) -> usize;
}

pub trait IntoVariant<V>: PayloadSize + Sized {
    fn into_variant(self) -> Result<V, Self>;
}

#[derive(Debug)]
pub enum Payload<P> {
    Raw(Raw<P>),
    Serialized(Serialized<P>),
}

impl<P> Payload<P> {
    pub fn new_raw<V>(data: V) -> Self 
    where
        P: Variant<V>,
        V: any::Any + Send,
    {
        Self::Raw(Raw::new(data))
    }

    pub fn new_serialized<V>(data: &V) -> Result<Self, SerializeError>
    where 
        P: Variant<V>,
        V: serde::Serialize,
    {
        Ok(Self::Serialized(Serialized::new(data)?))
    }
}

impl<P> private::Sealed for Payload<P> { }

impl<P> PayloadSize for Payload<P> {
    type Payload = P;

    fn byte_len(&self) -> usize {
        match self {
            Payload::Raw(r) => r.byte_len(),
            Payload::Serialized(s) => s.byte_len(),
        }
    }
}

impl<P, V> IntoVariant<V> for Payload<P>
where
    P: Variant<V>,
    V: serde::de::DeserializeOwned + any::Any + Send,
{
    fn into_variant(self) -> Result<V, Self> {
        match self {
            Payload::Raw(r) => 
                r.into_variant()
                    .map_err(|r| Self::Raw(r)),
            Payload::Serialized(s) => 
                s.into_variant()
                    .map_err(|s| Self::Serialized(s)),
        }
    }
}

#[derive(Debug)]
pub struct Raw<P> {
    // `: Send` can possibly be relaxed once moro has !Send futures
    // https://github.com/nikomatsakis/moro/pull/7
    data: Box<dyn any::Any + Send>,
    _p: PhantomData<fn (P) -> P>,
}

impl<P> Raw<P> {
    pub fn new<V>(data: V) -> Self 
    where
        P: Variant<V>,
        V: any::Any + Send,
    {
        Self {
            data: Box::new(data),
            _p: PhantomData::default(),
        }
    }
}

impl<P> private::Sealed for Raw<P> { }

impl<P> PayloadSize for Raw<P> {
    type Payload = P;

    fn byte_len(&self) -> usize {
        mem::size_of_val(&*self.data)
    }
}

impl<P, V> IntoVariant<V> for Raw<P>
where 
    P: Variant<V>,
    V: any::Any + Send,
{
    fn into_variant(self) -> Result<V, Self> {
        let Self {
            data,
            _p,
        } = self;
        data.downcast()
            .map(|v| *v)
            .map_err(|data| Self { data, _p })
    }
}

#[derive(Debug)]
pub struct Serialized<P> {
    data: Box<[u8]>,
    _p: PhantomData<fn (P) -> P>,
}

impl<P> Serialized<P> {
    pub fn new<V>(data: &V) -> Result<Self, SerializeError> 
    where
        P: Variant<V>,
        V: serde::Serialize,
    {
        let mut buffer = vec![];
        
        data.serialize(&mut rmp_serde::Serializer::new(&mut buffer))
            .map_err(SerializeError::from)?;

        Ok(Self {
            data: buffer.into_boxed_slice(),
            _p: PhantomData::default(),
        })
    }

    pub(crate) fn from_bytes(data: Box<[u8]>) -> Self {
        Self {
            data,
            _p: PhantomData::default(),
        }
    }

    pub fn into_bytes(self) -> Box<[u8]> {
        self.data
    }
}

impl<P> private::Sealed for Serialized<P> { }

impl<P> PayloadSize for Serialized<P> {
    type Payload = P;

    fn byte_len(&self) -> usize {
        self.data.len()
    }
}

impl<P, V> IntoVariant<V> for Serialized<P> 
where
    P: Variant<V>,
    V: serde::de::DeserializeOwned,
{
    fn into_variant(self) -> Result<V, Self> {
        match V::deserialize(&mut rmp_serde::Deserializer::new(&*self.data)) {
            Ok(value) => Ok(value),
            Err(_) => Err(self),
        }
    }
}
