use std::{marker::PhantomData, ops::DerefMut};

use bevy::prelude;

use crate::{client, common};

pub fn run_client<Client: crate::client::ClientSSS>(
    world: &mut prelude::World,
    q_client: &mut prelude::QueryState<(
        prelude::Entity,
        &mut super::component::Runner<Client>,
        &common::component::GameId,
        &super::component::ClientId,
    )>,
) -> super::RunClientResult<()> {
    // We need to access the Client components, but also need
    // full World access for Storage.
    // First create a list of all Client entities. Then query
    // each Entity individually, temporarily remove the needed
    // Components, create and use the Storage, then finally
    // restore the Components before returning any error.

    // This currently triggers add/remove component events
    // https://github.com/bevyengine/bevy/issues/13128

    let entities: Vec<_> = q_client.iter(world).map(|tup| tup.0).collect();

    let mut command_queue = bevy::ecs::world::CommandQueue::default();

    for entity in entities {
        let mut entity_mut = world.entity_mut(entity);
        let mut bundle: (
            super::component::Runner<Client>,
            super::component::EntityMap,
            super::component::FromServer<Client>,
            super::component::ToServer<Client>,
            super::component::FromUser<Client>,
            common::component::GameId,
            super::component::ClientId,
        ) = entity_mut.take().expect("Clients must remain valid");

        let (runner, entity_map, from_server, to_server, from_user, game_id, client_id) =
            &mut bundle;

        if !from_server.is_empty() || !from_user.is_empty() {
            prelude::trace!(
                "[{game_id}] Client {client_id} processing {} signals and {} user inputs",
                from_server.len(),
                from_user.len()
            );
        }

        let mut storage =
            super::storage::BevyStorage::new(world, entity_map, **game_id, **client_id);

        while let Some(client_signal) = from_server.dequeue() {
            // Manual reborrows are needed because the sub-functions take impl DerefMut instead of &mut
            let runner = &mut *runner;
            let to_server = &mut *to_server;

            signal(
                entity, 
                *game_id,
                *client_id,
                client_signal,
                &mut storage,
                runner,
                to_server,
                &mut command_queue,
            );
        }
        while let Some(user_input) = from_user.dequeue() {
            // Manual reborrows are needed because the sub-functions take impl DerefMut instead of &mut
            let runner = &mut *runner;
            let to_server = &mut *to_server;

            use crate::client::component::UserInput;
            match user_input {
                UserInput::StageInteraction(interaction) => {
                    stage_interaction(
                        entity, 
                        *game_id,
                        *client_id,
                        interaction,
                        &mut storage,
                        runner,
                        to_server,
                        &mut command_queue,
                    );
                }
                UserInput::ApplyInteraction(pending) => {
                    apply_interactions(
                        entity, 
                        *game_id,
                        *client_id,
                        pending,
                        &mut storage,
                        runner,
                        to_server,
                        &mut command_queue,
                    );
                }
                UserInput::RevertInteraction(pending) => {
                    revert_interactions(
                        entity, 
                        *game_id,
                        *client_id,
                        pending,
                        &mut storage,
                        runner,
                        to_server,
                        &mut command_queue,
                    );
                }
            }
        }

        // Return the component bundle to the client entity
        let mut client_entity = world.entity_mut(entity);
        client_entity.insert(bundle);

        command_queue.apply(world);
    }

    Ok(())
}

pub(crate) fn process_output<Client: super::ClientSSS, Ret>(
    entity: prelude::Entity,
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    output: spru::client::Output<Client, Ret>,
    mut to_server: impl std::ops::DerefMut<Target = super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) -> Ret {
    let spru::client::Output {
        outbound,
        events,
        ret,
    } = output;

    for server_signal in outbound {
        to_server.enqueue(server_signal);
    }

    for event in events {
        #[allow(clippy::single_match)]
        match event {
            spru::client::Event::GameComplete(game_complete) => {
                event_trigger.trigger(super::event::GameOutcome::<Client> {
                    entity,
                    game_id,
                    client_id,
                    game_outcome: game_complete.game_outcome,
                });
            }
            _ => {}
        }
    }

    ret
}

pub(crate) fn init<Client: super::ClientSSS>(
    init: spru::common::Seed<Client::Common>,

    world: &mut prelude::World,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let game_id = common::component::GameId::new(init.game_id());
    let result = (|| {
        let mut entity_map = super::component::EntityMap::default();
        let mut storage = super::storage::BevyStorage::new(
            world,
            &mut entity_map,
            init.game_id(),
            init.local_player_id(),
        );
        let client = Client::init(&mut storage, init)?;
        let client_id = super::component::ClientId::new(client.local_player_id());
        let root = common::component::Root::<Client::Common>::new(client.root().clone());

        let entity = world.spawn((
            game_id,
            client_id,
            prelude::Name::new(format!(
                "[{}:{}] spru client",
                game_id.friendly_display(),
                client_id
            )),
            entity_map,
            super::component::Runner::new(client),
            root,
        )).id();

        Ok(client::ClientInfo { entity, client_id })
    })();

    event_trigger.trigger(super::event::Init::<Client> {
        game_id,
        result,
        _client: PhantomData,
    });
}

pub(crate) fn signal<Client: super::ClientSSS>(
    entity: prelude::Entity,
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    signal: spru::common::signal::ToClient<Client::Common>,

    storage: &mut super::BevyStorage<Client::State>,
    mut runner: impl DerefMut<Target = super::component::Runner<Client>>,
    to_server: impl DerefMut<Target = super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner.client.signal(storage, signal)?;
        let () = process_output(entity, game_id, client_id, output, to_server, event_trigger);
        Ok(())
    })();

    event_trigger.trigger(super::event::Signal::<Client> {
        entity,
        game_id,
        client_id,
        result,
        _client: PhantomData,
    });
}

pub(crate) fn stage_interaction<Client: super::ClientSSS<Interaction: Clone>>(
    entity: prelude::Entity,
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    interaction: Client::Interaction,

    storage: &mut super::BevyStorage<Client::State>,
    mut runner: impl DerefMut<Target = super::component::Runner<Client>>,
    to_server: impl DerefMut<Target = super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let interaction_clone = interaction.clone();
    let result = (|| {
        let output = runner
            .client
            .stage_interaction(storage, interaction_clone)?;
        let pending_interaction_id =
            process_output(entity, game_id, client_id, output, to_server, event_trigger);
        Ok(pending_interaction_id)
    })();

    event_trigger.trigger(super::event::StageInteraction::<Client> {
        entity,
        game_id,
        client_id,
        interaction,
        result,
    });
}

pub(crate) fn apply_interactions<Client: super::ClientSSS>(
    entity: prelude::Entity,
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    pending_interaction_id: Option<spru::interaction::Pending>,

    storage: &mut super::BevyStorage<Client::State>,
    mut runner: impl DerefMut<Target = super::component::Runner<Client>>,
    to_server: impl DerefMut<Target = super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner
            .client
            .apply_interactions(storage, pending_interaction_id)?;
        let count = process_output(entity, game_id, client_id, output, to_server, event_trigger);

        Ok(count)
    })();

    event_trigger.trigger(super::event::ApplyInteractions::<Client> {
        entity,
        game_id,
        client_id,
        pending_interaction_id,
        result,
        _client: PhantomData,
    });
}

pub(crate) fn revert_interactions<Client: super::ClientSSS>(
    entity: prelude::Entity,
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    pending_interaction_id: Option<spru::interaction::Pending>,

    storage: &mut super::BevyStorage<Client::State>,
    mut runner: impl DerefMut<Target = super::component::Runner<Client>>,
    to_server: impl DerefMut<Target = super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner
            .client
            .revert_interactions(storage, pending_interaction_id)?;
        let count = process_output(entity, game_id, client_id, output, to_server, event_trigger);
        Ok(count)
    })();

    event_trigger.trigger(super::event::RevertInteractions::<Client> {
        entity,
        game_id,
        client_id,
        pending_interaction_id,
        result,
        _client: PhantomData,
    });
}
