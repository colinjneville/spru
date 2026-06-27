pub mod command;
pub mod component;
pub mod event;
#[cfg(feature = "remote")]
#[cfg(not(target_family = "wasm"))]
pub mod remote;
pub mod resource;
mod plugin;
pub use plugin::Plugin;
pub mod system;

use bevy::prelude;
use derive_where::derive_where;

use crate::common;

/// A [spru::Server] whose constituent types are [Send] + [Sync] + `'static`
pub trait ServerSSS:
    spru::Server<
        State: spru::State<Repr: Send + Sync + 'static> + Send + Sync + 'static,
        Action: Send + Sync + 'static,
        Interaction: Send + Sync + 'static,
        PlayerInit: spru::player::Init<In: Send + Sync + 'static> + Send + Sync + 'static,
        Reaction: spru::Reaction<
            GameOutcome: Send + Sync + 'static,
            Trigger: Send + Sync + 'static,
        > + Send
                      + Sync
                      + 'static,
        Root: Send + Sync + 'static,
        Common: common::CommonSSS,
    > + Send
    + Sync
    + 'static
{
    /// Filter a query over &[common::component::GameId] to
    /// the specific entity containing a Server with the given id. Panics if the
    /// [bevy::ecs::query::QueryData] does not contain GameId.
    fn filter<'w, 's, D, F>(
        query: &'w mut bevy::ecs::system::Query<'_, 's, D, F>,
        game_id: common::component::GameId,
    ) -> Option<bevy::ecs::query::ROQueryItem<'w, 's, D>>
    where
        D: bevy::ecs::query::QueryData,
        F: bevy::ecs::query::QueryFilter,
    {
        let id_lens: bevy::ecs::system::QueryLens<
            (prelude::Entity, &common::component::GameId),
            prelude::With<component::Runner<Self>>,
        > = query.transmute_lens_filtered();

        for (entity, &server_game_id) in id_lens.query_inner() {
            if server_game_id == game_id {
                return query.get(entity).ok();
            }
        }
        None
    }

    /// Filter a query over &[common::component::GameId] to
    /// the specific entity containing a Server with the given id. Panics if the
    /// [bevy::ecs::query::QueryData] does not contain GameId.
    fn filter_mut<'w, 's, D, F>(
        query: &'w mut bevy::ecs::system::Query<'_, 's, D, F>,
        game_id: common::component::GameId,
    ) -> Option<D::Item<'w, 's>>
    where
        D: bevy::ecs::query::QueryData,
        F: bevy::ecs::query::QueryFilter,
    {
        let id_lens: bevy::ecs::system::QueryLens<
            (prelude::Entity, &common::component::GameId),
            prelude::With<component::Runner<Self>>,
        > = query.transmute_lens_filtered();

        for (entity, &server_game_id) in id_lens.query_inner() {
            if server_game_id == game_id {
                return query.get_mut(entity).ok();
            }
        }
        None
    }
}

impl<
    Server: spru::Server<
            State: spru::State<Repr: Send + Sync + 'static> + Send + Sync + 'static,
            Action: Send + Sync + 'static,
            Interaction: Send + Sync + 'static,
            PlayerInit: spru::player::Init<In: Send + Sync + 'static> + Send + Sync + 'static,
            Reaction: spru::Reaction<
                GameOutcome: Send + Sync + 'static,
                Trigger: Send + Sync + 'static,
            > + Send
                          + Sync
                          + 'static,
            Root: Send + Sync + 'static,
            Common: common::CommonSSS,
        > + Send
        + Sync
        + 'static,
> ServerSSS for Server
{
}

#[derive(Debug, thiserror::Error)]
#[error("An error occurred while running a Server: {0}")]
pub enum RunServerError {
    Signal(#[from] spru::server::error::SignalError),
    AddPlayer(#[from] spru::server::error::AddPlayerError),
    ManualTrigger(#[from] spru::server::error::ManualTriggerError),
    CreateSave(#[from] spru::server::error::SaveError),
}

pub type RunServerResult<T> = std::result::Result<T, RunServerError>;

pub(crate) fn trigger_events<Server: ServerSSS>(entity: prelude::Entity, event_trigger: &mut impl common::TriggerEvent, events: Vec<spru::server::Event<Server>>) {
    for event in events {
        #[allow(clippy::single_match)]
        match event {
            spru::server::Event::GameComplete(game_complete) => {
                event_trigger.trigger(event::GameComplete::<Server> {
                    entity,
                    game_outcome: game_complete.game_outcome,
                });
            }
            _ => {}
        }
    }
}
