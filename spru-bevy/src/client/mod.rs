pub mod command;
pub mod component;
pub mod event;
mod lookup;
use bevy::prelude;
pub(crate) use lookup::BevyLookup;
mod plugin;
pub use plugin::Plugin;
use spru::item;

use crate::common;
pub mod system;

pub trait ClientSSS: 
    spru::client::Client<
        State: Send + Sync + 'static,
        Action: spru::Action<State = Self::State> + Send + Sync + 'static,
        GameOutcome: Send + Sync + 'static,
        Interaction: Clone + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        Common: crate::common::CommonSSS,
    > + Send + Sync + 'static
{
    /// Filter a query over &[`common::component::GameId`] and &[`component::ClientId`] to
    /// the specific entity containing a Client with the given ids. Panics if the 
    /// [bevy::ecs::query::QueryData] does not contain the id types.
    fn filter<'w, 's, D, F>(
        query: &'w mut bevy::ecs::system::Query<'_, 's, D, F>,
        game_id: common::component::GameId,
        client_id: component::ClientId,
    ) 
        -> Option<bevy::ecs::query::ROQueryItem<'w, 's, D>>
    where 
        D: bevy::ecs::query::QueryData,
        F: bevy::ecs::query::QueryFilter,
    {
        let id_lens: bevy::ecs::system::QueryLens<(
            prelude::Entity,
            &common::component::GameId,
            &component::ClientId,
        ),
            prelude::With<component::Runner<Self>>
        > = query.transmute_lens_filtered();

        for (entity, &client_game_id, &client_client_id) in id_lens.query_inner() {
            if client_game_id == game_id && client_client_id == client_id {
                return query.get(entity)
                    .ok();
            }
        }
        None
    }

    /// Filter a query over &[`common::component::GameId`] and &[`component::ClientId`] to
    /// the specific entity containing a Client with the given ids. Panics if the 
    /// [bevy::ecs::query::QueryData] does not contain the id types.
    fn filter_mut<'w, 's, D, F>(
        query: &'w mut bevy::ecs::system::Query<'_, 's, D, F>,
        game_id: common::component::GameId,
        client_id: component::ClientId,
    ) 
        -> Option<D::Item<'w, 's>>
    where 
        D: bevy::ecs::query::QueryData,
        F: bevy::ecs::query::QueryFilter,
    {
        let id_lens: bevy::ecs::system::QueryLens<(
            prelude::Entity,
            &common::component::GameId,
            &component::ClientId,
        ),
            prelude::With<component::Runner<Self>>
        > = query.transmute_lens_filtered();

        for (entity, &client_game_id, &client_client_id) in id_lens.query_inner() {
            if client_game_id == game_id && client_client_id == client_id {
                return query.get_mut(entity)
                    .ok();
            }
        }
        None
    }
}

impl<Client:
    spru::client::Client<
        State: Send + Sync + 'static,
        Action: spru::Action<State = Self::State> + Send + Sync + 'static,
        GameOutcome: Send + Sync + 'static,
        Interaction: Clone + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        Common: crate::common::CommonSSS,
    > + Send + Sync + 'static
> ClientSSS for Client { }


#[derive(Debug, Clone)]
#[derive(thiserror::Error)]
pub enum BevyError {
    #[error("Item {0} does not exist")]
    IdNotFound(item::Id),
    #[error("Item {0} should exist, but the bevy entity ({1}) has been removed")]
    EntityNotFound(item::Id, prelude::Entity),
    #[error("Item {0} should exist, but the bevy component ({1} {2}) has been removed")]
    ComponentNotFound(item::Id, prelude::Entity, &'static str),
    #[error("Item {0} already exists")]
    IdAlreadyExists(item::Id, prelude::Entity),
}

pub type BevyResult<T> = std::result::Result<T, BevyError>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum RunClientError {
    #[error(transparent)]
    Init(spru::common::error::FatalError),
    #[error(transparent)]
    StageInteraction(#[from] spru::client::error::StageInteractionError),
    #[error(transparent)]
    ApplyInteraction(#[from] spru::client::error::ApplyInteractionError),
    #[error(transparent)]
    RevertInteraction(#[from] spru::client::error::RevertInteractionError),
    #[error(transparent)]
    Signal(spru::common::error::FatalError),
}

pub type RunClientResult<T> = std::result::Result<T, RunClientError>;
