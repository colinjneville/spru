use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::{client, common};

#[derive_where(Debug; spru::common::Seed<Client::Common>)]
pub struct Init<Client: super::ClientSSS> {
    pub seed: spru::common::Seed<Client::Common>,
}

impl<Client: super::ClientSSS> prelude::EntityCommand for Init<Client> {
    fn apply(self, mut entity: prelude::EntityWorldMut) {
        let Self { seed } = self;

        let game_id = common::component::GameId::new(seed.game_id());
        let result = (|| {
            let mut entity_map = super::component::EntityMap::default();
            let client = entity.world_scope(|world| {
                let mut storage = super::storage::BevyStorage::new(
                    world,
                    &mut entity_map,
                    seed.game_id(),
                    seed.local_player_id(),
                );
                Client::init(&mut storage, seed)
            })?;
            let client_id = super::component::ClientId::new(client.local_player_id());
            let root = common::component::Root::<Client::Common>::new(client.root().clone());

            entity.insert((
                game_id,
                client_id,
                prelude::Name::new(format!(
                    "[{}:{}] spru client",
                    game_id.friendly_display(),
                    client_id
                )),
                entity_map,
                client::component::Runner::new(client),
                root,
            ));

            Ok(client_id)
        })();

        entity.trigger(|entity| client::event::Init::<Client> {
            entity,
            game_id,
            result,
            _client: PhantomData,
        });
    }
}

#[derive(Debug)]
pub struct StageInteraction<Client: client::ClientSSS> {
    interaction: Client::Interaction,
}

impl<Client: client::ClientSSS> StageInteraction<Client> {
    pub fn new(interaction: Client::Interaction) -> Self {
        Self {
            interaction,
        }
    }
}

impl<Client: client::ClientSSS> prelude::EntityCommand<prelude::Result> for StageInteraction<Client> {
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> prelude::Result {
        let Self {
            interaction,
        } = self;

        let result = match client::component::Runner::<Client>::storage_scope(&mut entity, |client, storage| {
            client.stage_interaction(storage, interaction.clone())
        })? {
            Ok(output) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret,
                } = output;

                entity.get_components_mut::<&mut client::component::ToServer<Client>>()?
                    .enqueue_outbound(outbound);
                client::trigger_events(entity.id(), &mut entity, events);

                Ok(ret)
            },
            Err(err) => Err(err),
        };

        entity.trigger(|entity| super::event::StageInteraction::<Client> {
            entity,
            interaction,
            result,
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct ApplyInteractions<Client: client::ClientSSS> {
    pending_interaction_id: Option<spru::interaction::Pending>,
    _p: PhantomData<Client>,
}

impl<Client: client::ClientSSS> ApplyInteractions<Client> {
    pub fn all() -> Self {
        Self {
            pending_interaction_id: None,
            _p: PhantomData,
        }
    }

    pub fn new(pending_interaction_id: spru::interaction::Pending) -> Self {
        Self {
            pending_interaction_id: Some(pending_interaction_id),
            _p: PhantomData,
        }
    }
}

impl<Client: client::ClientSSS> prelude::EntityCommand<prelude::Result> for ApplyInteractions<Client> {
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> prelude::Result {
        let Self {
            pending_interaction_id,
            _p,
        } = self;

        let result = match client::component::Runner::<Client>::storage_scope(&mut entity, |client, storage| {
            client.apply_interactions(storage, pending_interaction_id)
        })? {
            Ok(output) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret,
                } = output;

                entity.get_components_mut::<&mut client::component::ToServer<Client>>()?
                    .enqueue_outbound(outbound);
                client::trigger_events(entity.id(), &mut entity, events);

                Ok(ret)
            },
            Err(err) => Err(err),
        };

        entity.trigger(|entity| super::event::ApplyInteractions::<Client> {
            entity,
            pending_interaction_id,
            result,
            _client: PhantomData,
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct RevertInteractions<Client: client::ClientSSS> {
    pending_interaction_id: Option<spru::interaction::Pending>,
    _p: PhantomData<Client>,
}

impl<Client: client::ClientSSS> RevertInteractions<Client> {
    pub fn all() -> Self {
        Self {
            pending_interaction_id: None,
            _p: PhantomData,
        }
    }

    pub fn new(pending_interaction_id: spru::interaction::Pending) -> Self {
        Self {
            pending_interaction_id: Some(pending_interaction_id),
            _p: PhantomData,
        }
    }
}

impl<Client: client::ClientSSS> prelude::EntityCommand<prelude::Result> for RevertInteractions<Client> {
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> prelude::Result {
        let Self {
            pending_interaction_id,
            _p,
        } = self;

        let result = match client::component::Runner::<Client>::storage_scope(&mut entity, |client, storage| {
            client.revert_interactions(storage, pending_interaction_id)
        })? {
            Ok(output) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret,
                } = output;

                entity.get_components_mut::<&mut client::component::ToServer<Client>>()?
                    .enqueue_outbound(outbound);
                client::trigger_events(entity.id(), &mut entity, events);

                Ok(ret)
            },
            Err(err) => Err(err),
        };

        entity.trigger(|entity| client::event::RevertInteractions::<Client> {
            entity,
            pending_interaction_id,
            result,
            _client: PhantomData,
        });

        Ok(())
    }
}
