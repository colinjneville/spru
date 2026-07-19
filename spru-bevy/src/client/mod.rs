pub mod command;
pub mod component;
pub mod event;
#[cfg(feature = "remote")]
pub mod remote;
pub mod resource;
mod storage;
pub use storage::BevyStorage;
mod plugin;
pub use plugin::Plugin;
pub mod system;

use spru::item;
use bevy::prelude;

use crate::common;

/// A [spru::Client] whose constituent types are [Send] + [Sync] + `'static`
pub trait ClientSSS:
    spru::client::Client<
        State: spru::State<Repr: Send + Sync + 'static> + Send + Sync + 'static,
        Action: spru::Action<State = Self::State> + Send + Sync + 'static,
        GameOutcome: Send + Sync + 'static,
        Interaction: Clone + Send + Sync + 'static,
        Root: Send + Sync + 'static,
        Common: crate::common::CommonSSS,
    > + Send
    + Sync
    + 'static
{
    /// Filter a query over &[common::component::GameId] and &[component::ClientId] to
    /// the specific entity containing a Client with the given ids. Panics if the
    /// [bevy::ecs::query::QueryData] does not contain the id types.
    fn filter<'w, 's, D, F>(
        query: &'w mut bevy::ecs::system::Query<'_, 's, D, F>,
        game_id: common::component::GameId,
        client_id: component::ClientId,
    ) -> Option<bevy::ecs::query::ROQueryItem<'w, 's, D>>
    where
        D: bevy::ecs::query::QueryData,
        F: bevy::ecs::query::QueryFilter,
    {
        let id_lens: bevy::ecs::system::QueryLens<
            (
                prelude::Entity,
                &common::component::GameId,
                &component::ClientId,
            ),
            prelude::With<component::Runner<Self>>,
        > = query.transmute_lens_filtered();

        for (entity, &client_game_id, &client_client_id) in id_lens.query_inner() {
            if client_game_id == game_id && client_client_id == client_id {
                return query.get(entity).ok();
            }
        }
        None
    }

    /// Filter a query over &[common::component::GameId] and &[component::ClientId] to
    /// the specific entity containing a Client with the given ids. Panics if the
    /// [bevy::ecs::query::QueryData] does not contain the id types.
    fn filter_mut<'w, 's, D, F>(
        query: &'w mut bevy::ecs::system::Query<'_, 's, D, F>,
        game_id: common::component::GameId,
        client_id: component::ClientId,
    ) -> Option<D::Item<'w, 's>>
    where
        D: bevy::ecs::query::QueryData,
        F: bevy::ecs::query::QueryFilter,
    {
        let id_lens: bevy::ecs::system::QueryLens<
            (
                prelude::Entity,
                &common::component::GameId,
                &component::ClientId,
            ),
            prelude::With<component::Runner<Self>>,
        > = query.transmute_lens_filtered();

        for (entity, &client_game_id, &client_client_id) in id_lens.query_inner() {
            if client_game_id == game_id && client_client_id == client_id {
                return query.get_mut(entity).ok();
            }
        }
        None
    }
}

impl<
    Client: spru::client::Client<
            State: spru::State<Repr: Send + Sync + 'static> + Send + Sync + 'static,
            Action: spru::Action<State = Self::State> + Send + Sync + 'static,
            GameOutcome: Send + Sync + 'static,
            Interaction: Clone + Send + Sync + 'static,
            Root: Send + Sync + 'static,
            Common: crate::common::CommonSSS,
        > + Send
        + Sync
        + 'static,
> ClientSSS for Client
{
}

#[derive(Debug, Clone, thiserror::Error)]
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

#[cfg(feature = "script")]
pub fn eval<Client, Args, Ret>(
    world: &prelude::World,
    client_entity: prelude::Entity,
    language: &impl spru_script::DialectEval<Args, Ret, Client::Root, Error: std::error::Error + Send + Sync + 'static>,
    script: &str,
    args: Args,
) 
    -> prelude::Result<Ret> 
where
    Client: ClientSSS,
{
    let (root, entity_map) = world.get_entity(client_entity)?
        .components::<(
            &common::component::Root<Client::Common>,
            &component::EntityMap,
        )>();

    let storage = storage::BevyReadOnlyStorage::new(world, entity_map);

    let ret = language.eval(&storage, &root.0, script, args)?;
    Ok(ret)
}

pub trait IdTExt {
    type T;

    fn try_from_world<'w>(&self, world: &'w prelude::World, client_entity: prelude::Entity) -> Option<&'w Self::T>;

    fn from_world<'w>(&self, world: &'w prelude::World, client_entity: prelude::Entity) -> &'w Self::T {
        self.try_from_world(world, client_entity)
            .expect("Item does not exist")
    }
}

impl<T: Send + Sync + 'static> IdTExt for spru::item::IdT<T> {
    type T = T;

    fn try_from_world<'w>(&self, world: &'w prelude::World, client_entity: prelude::Entity) -> Option<&'w Self::T> {
        let entity_map = world.entity(client_entity)
            .get_components::<&component::EntityMap>()
            .ok()?;
        let item_entity = entity_map.get(*self)
            .ok()?;
        let item = world.entity(item_entity)
            .get_components::<&component::Item<T>>()
            .ok()?;

        Some(&**item)
    }
}

fn trigger_events<Client: ClientSSS>(entity: prelude::Entity, event_trigger: &mut impl common::TriggerEvent, events: Vec<spru::client::Event<Client>>) {
    for event in events {
        #[allow(clippy::single_match)]
        match event {
            spru::client::Event::InteractionEvaluated(interaction_evaluated) => {
                // TODO
            }
            spru::client::Event::PlayerAdded(player_added) => {
                event_trigger.trigger(event::PlayerAdded {
                    entity,
                    player_id: player_added.player_id,
                });
            }
            spru::client::Event::PlayerRemoved(player_removed) => {
                event_trigger.trigger(event::PlayerRemoved {
                    entity,
                    player_id: player_removed.player_id,
                });
            }
            spru::client::Event::GameCompleted(game_complete) => {
                event_trigger.trigger(event::GameComplete::<Client> {
                    entity,
                    game_outcome: game_complete.game_outcome,
                });
            }
            _ => {}
        }
    }
}
