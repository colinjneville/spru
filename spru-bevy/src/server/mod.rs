pub mod command;
pub mod component;
pub mod event;
mod plugin;
pub use plugin::Plugin;
pub mod system;

use bevy::prelude;

use crate::common;

pub trait ServerSSS: 
    spru::Server<
        State: spru::State + Send + Sync + 'static,
        Action: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        PlayerInit: spru::player::Init<In: Send + Sync + 'static> + Send + Sync + 'static,
        Reaction: spru::Reaction<GameOutcome: Send + Sync + 'static, Trigger: Send + Sync + 'static> + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        Common: common::CommonSSS,
    > + Send + Sync + 'static
{
    /// Filter a query over &[`common::component::GameId`] to
    /// the specific entity containing a Server with the given id. Panics if the 
    /// [bevy::ecs::query::QueryData] does not contain GameId.
    fn filter<'w, 's, D, F>(
        query: &'w mut bevy::ecs::system::Query<'_, 's, D, F>,
        game_id: common::component::GameId,
    ) 
        -> Option<bevy::ecs::query::ROQueryItem<'w, 's, D>>
    where 
        D: bevy::ecs::query::QueryData,
        F: bevy::ecs::query::QueryFilter,
    {
        let id_lens: bevy::ecs::system::QueryLens<(
            prelude::Entity,
            &common::component::GameId,
        ),
            prelude::With<component::Runner<Self>>
        > = query.transmute_lens_filtered();

        for (entity, &server_game_id) in id_lens.query_inner() {
            if server_game_id == game_id {
                return query.get(entity)
                    .ok();
            }
        }
        None
    }

    /// Filter a query over &[`common::component::GameId`] to
    /// the specific entity containing a Server with the given id. Panics if the 
    /// [bevy::ecs::query::QueryData] does not contain GameId.
    fn filter_mut<'w, 's, D, F>(
        query: &'w mut bevy::ecs::system::Query<'_, 's, D, F>,
        game_id: common::component::GameId,
    ) 
        -> Option<D::Item<'w, 's>>
    where 
        D: bevy::ecs::query::QueryData,
        F: bevy::ecs::query::QueryFilter,
    {
        let id_lens: bevy::ecs::system::QueryLens<(
            prelude::Entity,
            &common::component::GameId,
        ),
            prelude::With<component::Runner<Self>>
        > = query.transmute_lens_filtered();

        for (entity, &server_game_id) in id_lens.query_inner() {
            if server_game_id == game_id {
                return query.get_mut(entity)
                    .ok();
            }
        }
        None
    }
}

impl<Server:
    spru::Server<
        State: spru::State + Send + Sync + 'static,
        Action: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        PlayerInit: spru::player::Init<In: Send + Sync + 'static> + Send + Sync + 'static,
        Reaction: spru::Reaction<GameOutcome: Send + Sync + 'static, Trigger: Send + Sync + 'static> + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        Common: common::CommonSSS,
    > + Send + Sync + 'static
> ServerSSS for Server { }

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum RunServerError {
    #[error(transparent)]
    Signal(#[from] spru::server::signal::Error),
    #[error(transparent)]
    AddPlayer(#[from] spru::server::add_player::Error),
    #[error(transparent)]
    ManualTrigger(#[from] spru::server::manual_trigger::Error),
    #[error(transparent)]
    CreateSave(#[from] spru::server::save::Error),
}

pub type RunServerResult<T> = std::result::Result<T, RunServerError>;


