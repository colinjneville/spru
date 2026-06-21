pub mod spru {
    pub mod player {
        #[bevy::reflect::reflect_remote(spru::player::Id)]
        #[reflect(opaque)]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        pub struct Id(u32);
    }

    pub mod game {
        #[bevy::reflect::reflect_remote(spru::game::Id)]
        #[reflect(opaque)]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        pub struct Id(uuid::Uuid);
    }

    pub mod item {
        use derive_where::derive_where;
        
        #[bevy::reflect::reflect_remote(spru::item::Id)]
        #[reflect(opaque)]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        pub struct Id(u32);

        #[bevy::reflect::reflect_remote(spru::item::IdT<T>)]
        #[reflect(opaque)]
        #[derive_where(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize; )]
        pub struct IdT<T>(u32, PhantomData<T>);
    }
}


