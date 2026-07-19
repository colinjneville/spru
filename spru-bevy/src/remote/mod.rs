pub mod component;

pub(crate) const SERVER_TO_CLIENT_LANES: [aeronet::transport::lane::LaneKind; 2] = [
    // Connection coordination
    aeronet::transport::lane::LaneKind::ReliableOrdered,
    // spru signals
    aeronet::transport::lane::LaneKind::ReliableOrdered,
];

pub(crate) const CLIENT_TO_SERVER_LANES: [aeronet::transport::lane::LaneKind; 2] = [
    // Connection coordination
    aeronet::transport::lane::LaneKind::ReliableOrdered,
    // spru signals
    aeronet::transport::lane::LaneKind::ReliableOrdered,
];

pub(crate) const SERVER_TO_CLIENT_LANE_COORDINATION: aeronet::transport::lane::LaneIndex = aeronet::transport::lane::LaneIndex::new(0);
pub(crate) const SERVER_TO_CLIENT_LANE_SIGNAL: aeronet::transport::lane::LaneIndex = aeronet::transport::lane::LaneIndex::new(1);

pub(crate) const CLIENT_TO_SERVER_LANE_COORDINATION: aeronet::transport::lane::LaneIndex = aeronet::transport::lane::LaneIndex::new(0);
pub(crate) const CLIENT_TO_SERVER_LANE_SIGNAL: aeronet::transport::lane::LaneIndex = aeronet::transport::lane::LaneIndex::new(1);

// TODO the goal is to completely wrap these so they are not exposed
pub use aeronet;
pub use aeronet_webtransport;

use std::fmt;

#[derive(Debug, Clone)]
pub enum DisconnectedReason {
    ByUser(String),
    ByPeer(String),
    ByError(String),
}

impl DisconnectedReason {
    pub fn by_user(&self) -> Option<&str> {
        if let Self::ByUser(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn by_peer(&self) -> Option<&str> {
        if let Self::ByPeer(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn by_error(&self) -> Option<&str> {
        if let Self::ByError(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub(crate) fn from_aeronet(reason: &aeronet::io::connection::DisconnectReason) -> Self {
        match reason {
            aeronet::io::connection::DisconnectReason::ByUser(s) => Self::ByUser(s.clone()),
            aeronet::io::connection::DisconnectReason::ByPeer(s) => Self::ByPeer(s.clone()),
            aeronet::io::connection::DisconnectReason::ByError(error) => Self::ByError(error.to_string()),
        }
    }
}

impl fmt::Display for DisconnectedReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisconnectedReason::ByUser(s) => write!(f, "Disconnected by user: {s}"),
            DisconnectedReason::ByPeer(s) => write!(f, "Disconnected by peer: {s}"),
            DisconnectedReason::ByError(s) => write!(f, "Disconnected due to error: {s}"),
        }
    }
}

pub(crate) type SerializeError = cfg_select! {
    feature = "traced-comms" => ron::Error,
    _ => rmp_serde::encode::Error,
};

/// Serialize the value. On debug builds, serialize as human-readable, and trace the result
pub(crate) fn serialize<T: serde::Serialize>(t: &T) -> Result<Vec<u8>, SerializeError> {
    cfg_select! {
        feature = "traced-comms" => {
            let mut buf = String::new();
            let mut serializer = ron::Serializer::new(&mut buf, None)?;
            t.serialize(&mut serializer)?;

            bevy::prelude::trace!(serialized = buf);

            Ok(buf.into_bytes())
        }
        _ => {
            let mut buf = vec![];
            let mut serializer = rmp_serde::Serializer::new(&mut buf);

            t.serialize(&mut serializer)?;

            Ok(buf)
        }
    }
}

pub(crate) type DeserializeError = cfg_select! {
    feature = "traced-comms" => ron::Error,
    _ => rmp_serde::decode::Error,
};

/// Deserialize the value. On debug builds, trace the input, deserialize as human-readable
pub(crate) fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, DeserializeError> {
    let mut deserializer = cfg_select! {
        feature = "traced-comms" => {
            {
                let s = str::from_utf8(&bytes)
                    .expect("Human-readable output should be valid Unicode");

                bevy::prelude::trace!(deserialized = s);

                ron::Deserializer::from_bytes(bytes)?
            }
        }
        _ => rmp_serde::Deserializer::new(bytes),
    };
    
    T::deserialize(&mut deserializer)
}
