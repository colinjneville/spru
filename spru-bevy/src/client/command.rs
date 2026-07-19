use std::{marker::PhantomData, sync::Arc};

use bevy::prelude;
use derive_where::derive_where;
use tracing::instrument;

use crate::{client, common};

#[derive_where(Debug; spru::common::Seed<Client::Common>)]
pub struct Init<Client: super::ClientSSS> {
    pub seed: spru::common::Seed<Client::Common>,
}

impl<Client: super::ClientSSS> prelude::EntityCommand for Init<Client> {
    type Out = ();

    #[instrument(skip_all)]
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

            // Keep this sync'ed with Shutdown
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

        match result {
            Ok(client_id) => {
                entity.trigger(|entity| client::event::Initialized {
                    entity,
                    game_id,
                    client_id,
                });
            }
            Err(error) => {
                entity.trigger(|entity| client::event::InitializeError {
                    entity,
                    game_id,
                    error: Arc::new(error),
                });
            }
        }
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

impl<Client: client::ClientSSS> prelude::EntityCommand for StageInteraction<Client> {
    type Out = prelude::Result;

    #[instrument(skip_all)]
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

        match result {
            Ok(pending_id) => {
                entity.trigger(|entity| super::event::InteractionStaged::<Client> {
                    entity,
                    interaction,
                    pending_id,
                });
            }
            Err(error) => {
                entity.trigger(|entity| super::event::InteractionStageError::<Client> {
                    entity,
                    interaction,
                    error: Arc::new(error),
                });
            }
        }

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

impl<Client: client::ClientSSS> prelude::EntityCommand for ApplyInteractions<Client> {
    type Out = prelude::Result;

    #[instrument(skip_all)]
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

        match result {
            Ok(count) => {
                entity.trigger(|entity| super::event::InteractionsApplied {
                    entity,
                    pending_interaction_id,
                    count,
                });
            }
            Err(error) => {
                entity.trigger(|entity| super::event::InteractionsApplyError {
                    entity,
                    pending_interaction_id,
                    error: Arc::new(error),
                });
            }
        }

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

impl<Client: client::ClientSSS> prelude::EntityCommand for RevertInteractions<Client> {
    type Out = prelude::Result;
    
    #[instrument(skip_all)]
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

        match result {
            Ok(count) => {
                entity.trigger(|entity| client::event::InteractionsReverted {
                    entity,
                    pending_interaction_id,
                    count,
                });
            }
            Err(error) => {
                entity.trigger(|entity| client::event::InteractionsRevertError {
                    entity,
                    pending_interaction_id,
                    error: Arc::new(error),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Shutdown<Client> {
    pub despawn: bool,
    _p: PhantomData<Client>,
}

impl<Client> Shutdown<Client> {
    pub fn new(despawn: bool) -> Self {
        Self {
            despawn,
            _p: PhantomData,
        }
    }
}

impl<Client: client::ClientSSS> prelude::EntityCommand for Shutdown<Client> {
    type Out = ();

    #[instrument(skip_all)]
    fn apply(self, mut entity: prelude::EntityWorldMut) -> Self::Out {
        let Self {
            despawn,
            _p,
        } = self;

        if entity.get_components::<&client::component::Runner<Client>>().is_err() {
            prelude::info!("No client detected, skipping shutdown");
            return;
        }

        let (game_id, client_id) = entity.components::<(&common::component::GameId, &client::component::ClientId)>();
        prelude::error_span!("Shutdown::apply", game_id = %**game_id, client_id = %**client_id, despawn);

        prelude::info!("Shutting down client");
        
        if client::component::Runner::<Client>::storage_scope(&mut entity, |client, storage| {
            if let Err(err) = client.shutdown(storage) {
                prelude::warn!("Failed to shutdown client cleanly: {err}");
            }
        }).is_err() {
            prelude::info!("Client appears to already have been shutdown");
            return;
        }

        if despawn {
            entity.despawn();
        } else {
            // Remove the functional components, leave markers like game_id, client_id
            entity.remove::<(
                client::component::Runner<Client>,
                client::component::EntityMap,
                client::component::FromServer<Client>,
                client::component::ToServer<Client>,
            )>();
        }
    }
}
