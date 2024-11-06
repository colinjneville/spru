use std::any;

use crate::{header, payload, Header, Payload, SerializeError};

#[derive(Debug)]
pub struct Message<P> {
    pub header: Header,
    pub payload: P,
    _p: (),
}

impl<T> Message<T> {
    pub fn into_variant<V>(self) -> Result<V, Error<T>> 
    where 
        T: payload::IntoVariant<V>,
        T::Payload: payload::Variant<V>,
    {
        if self.header.variant_id == <T::Payload as payload::Variant<V>>::variant_id()  {
            let Self {
                header,
                payload,
                _p: (),
            } = self;
    
            payload.into_variant()
                .map_err(|payload| Error::Cast(Self {
                    header,
                    payload,
                    _p: (),
                }))
        } else {
            Err(Error::Variant(self))
        }
    }
}

impl<V> Message<V> {
    pub fn into_raw<P>(self) -> Message<payload::Raw<P>>
    where 
        P: payload::Variant::<V>,
        V: any::Any + Send,
    {
        let Self {
            header,
            payload,
            _p,
        } = self;
        Message::<P>::new_raw(payload)
    }

    pub fn into_serialized<P>(&self) -> Result<Message<payload::Serialized<P>>, SerializeError>
    where 
        P: payload::Variant::<V>,
        V: serde::Serialize,
    {
        let Self {
            header,
            payload,
            _p,
        } = self;
        Message::<P>::new_serialized(payload)
    }
}

impl<P> Message<P> {
    pub fn new_raw<V>(data: V) -> Message<payload::Raw<P>>
    where 
        V: any::Any + Send,
        P: payload::Variant<V>,
    {
        use payload::PayloadSize as _;

        let payload = payload::Raw::<P>::new(data);

        let variant_id = P::variant_id();
        let header = Header {
            payload_size: payload.byte_len(),
            variant_id,
        };
        Message {
            header,
            payload,
            _p: (),
        }
    }

    pub fn new_serialized<V>(data: &V) -> Result<Message<payload::Serialized<P>>, crate::SerializeError> 
    where 
        V: serde::Serialize,
        P: payload::Variant<V>,
    {
        use payload::PayloadSize as _;

        let payload = payload::Serialized::<P>::new(data)?;
        let payload_size = payload.byte_len();

        let variant_id = P::variant_id();
        let header = Header {
            payload_size,
            variant_id,
        };

        Ok(Message {
            header,
            payload,
            _p: (),
        })
    }

    pub fn from_bytes(header: Header, data: Box<[u8]>) -> Message<payload::Serialized<P>> {
        Message {
            header,
            payload: payload::Serialized::from_bytes(data),
            _p: (),
        }
    }
}

impl<P> Message<payload::Serialized<P>> {
    pub fn into_bytes(self) -> Box<[u8]> {
        let mut output = vec![];
        output.copy_from_slice(&*self.header.to_bytes());
        output.copy_from_slice(&*self.payload.into_bytes());
        output.into_boxed_slice()
    }
}

impl<P> From<Message<payload::Raw<P>>> for Message<Payload<P>> {
    fn from(value: Message<payload::Raw<P>>) -> Self {
        Self {
            header: value.header,
            payload: Payload::Raw(value.payload),
            _p: (),
        }
    }
}

impl<P> From<Message<payload::Serialized<P>>> for Message<Payload<P>> {
    fn from(value: Message<payload::Serialized<P>>) -> Self {
        Self {
            header: value.header,
            payload: Payload::Serialized(value.payload),
            _p: (),
        }
    }
}

pub enum Error<P> {
    Variant(Message<P>),
    Cast(Message<P>),
}