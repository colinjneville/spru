pub mod spru {
    pub mod player {
        #[bevy::reflect::reflect_remote(spru::player::Id)]
        #[reflect(opaque)]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        pub struct Id;
    }

    pub mod game {
        #[bevy::reflect::reflect_remote(spru::game::Id)]
        #[reflect(opaque)]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        pub struct Id;
    }

    pub mod item {
        use derive_where::derive_where;
        
        #[bevy::reflect::reflect_remote(spru::item::Id)]
        #[reflect(opaque)]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        pub struct Id;

        #[bevy::reflect::reflect_remote(spru::item::IdT<T>)]
        #[reflect(opaque)]
        #[derive_where(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize; )]
        pub struct IdT<T>;
    }
}

#[cfg(feature = "remote")]
pub mod url {
    #[bevy::reflect::reflect_remote(url::Url)]
    #[reflect(opaque)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Url;
}
