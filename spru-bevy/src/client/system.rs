use std::{marker::PhantomData, ops::DerefMut};

use bevy::prelude;

use crate::common;

pub fn run_client<Client: crate::client::ClientSSS>(
    world: &mut prelude::World,
) 
    -> super::RunClientResult<()>
{
    // We need to access the Client components, but also need
    // full World access for Lookup.
    // First create a list of all Client entities. Then query
    // each Entity individually, temporarily remove the needed
    // Components, create and use the Lookup, then finally 
    // restore the Components before returning any error.

    // This currently triggers add/remove component events
    // https://github.com/bevyengine/bevy/issues/13128

    let mut q_client = world.query::<
        (
            prelude::Entity,
            &mut super::component::Runner<Client>, 
            &common::component::GameId,
            &super::component::ClientId,
        )
    >();

    let entities: Vec<_> = q_client.iter(world)
        .map(|tup| tup.0)
        .collect();

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
        ) = entity_mut.take()
            .expect("Clients must remain valid");

        let (
            runner,
            entity_map,
            from_server,
            to_server,
            from_user,
            game_id,
            client_id,
        ) = &mut bundle;

        if !from_server.is_empty() || !from_user.is_empty() {
            prelude::trace!("[{game_id}] Client {client_id} processing {} signals and {} user inputs", from_server.len(), from_user.len());
        }

        let mut lookup = super::lookup::BevyLookup::new(world, entity_map, game_id.0, client_id.0);

        while let Some(client_signal) = from_server.dequeue() {
            // Manual reborrows are needed because the sub-functions take impl DerefMut instead of &mut
            let runner = &mut *runner;
            let to_server = &mut *to_server;
            
            signal(*game_id, *client_id, client_signal, &mut lookup, runner, to_server, &mut command_queue);
        }
        while let Some(user_input) = from_user.dequeue() {
            // Manual reborrows are needed because the sub-functions take impl DerefMut instead of &mut
            let runner = &mut *runner;
            let to_server = &mut *to_server;

            use crate::client::component::UserInput;
            match user_input {
                UserInput::StageInteraction(interaction) => {
                    stage_interaction(*game_id, *client_id, interaction, &mut lookup, runner, to_server, &mut command_queue);
                }
                UserInput::ApplyInteraction(pending) => {
                    apply_interactions(*game_id, *client_id, pending, &mut lookup, runner, to_server, &mut command_queue);
                }
                UserInput::RevertInteraction(pending) => {
                    revert_interactions(*game_id, *client_id, pending, &mut lookup, runner, to_server, &mut command_queue);
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
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    output: spru::client::Output<Client, Ret>,
    mut to_server: impl std::ops::DerefMut<Target = super::component::ToServer::<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
)
    -> Ret
{
    let spru::client::Output {
        outbound,
        events,
        ret,
    } = output;

    for server_signal in outbound {
        to_server.enqueue(server_signal);
    }

    for event in events {
        match event {
            spru::client::Event::GameComplete(game_complete) => {
                event_trigger.trigger(super::event::GameOutcome::<Client> {
                    game_id,
                    client_id,
                    game_outcome: game_complete.game_outcome,
                });
            },
            _ => { },
        }
    }

    ret
}

pub(crate) fn init<Client: super::ClientSSS>(
    init: spru::client::init::Arg<Client::Common>,

    world: &mut prelude::World, 
    event_trigger: &mut impl common::TriggerEvent,
) {
    let game_id = common::component::GameId::new(init.game_id());
    let result = (|| {
        let mut entity_map = super::component::EntityMap::default();
        let mut lookup = super::lookup::BevyLookup::new(world, &mut entity_map, init.game_id(), init.local_player_id());
        let client = Client::init(&mut lookup, init)?;
        let client_id = super::component::ClientId(client.local_player_id());

        world.spawn((
            game_id,
            client_id,
            prelude::Name::new(format!("[{:x}:{}] spru client", game_id.0.friendly_display(), client_id)),
            entity_map,
            super::component::Runner::new(client),
        ));
        
        Ok(client_id)
    })();
    
    event_trigger.trigger(super::event::Init::<Client> {
        game_id,
        result,
        _client: PhantomData,
    });
}

pub(crate) fn signal<Client: super::ClientSSS>(
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    signal: spru::client::Signal<Client::Common>,

    lookup: &mut super::BevyLookup<Client::State>, 
    mut runner: impl DerefMut<Target=super::component::Runner<Client>>,
    to_server: impl DerefMut<Target=super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner.client.signal(lookup, signal)?;
        let spru::client::signal::Ret {
            
        } = process_output(game_id, client_id, output, to_server, event_trigger);
        Ok(())
    })();
    
    event_trigger.trigger(super::event::Signal::<Client> {
        game_id,
        client_id,
        result,
        _client: PhantomData,
    });
}

pub(crate) fn stage_interaction<Client: super::ClientSSS<Interaction: Clone>>(
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    interaction: Client::Interaction,

    lookup: &mut super::BevyLookup<Client::State>, 
    mut runner: impl DerefMut<Target=super::component::Runner<Client>>,
    to_server: impl DerefMut<Target=super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let interaction_clone = interaction.clone();
    let result = (|| {
        let output = runner.client.stage_interaction(lookup, interaction_clone)?;
        let spru::client::stage_interaction::Ret {
            pending_transaction_id,
        } = process_output(game_id, client_id, output, to_server, event_trigger);
        Ok(pending_transaction_id)
    })();
    
    event_trigger.trigger(super::event::StageInteraction::<Client> {
        game_id,
        client_id,
        interaction,
        result,
    });
}

pub(crate) fn apply_interactions<Client: super::ClientSSS>(
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    pending_transaction_id: Option<spru::transaction::Pending>,

    lookup: &mut super::BevyLookup<Client::State>, 
    mut runner: impl DerefMut<Target=super::component::Runner<Client>>,
    to_server: impl DerefMut<Target=super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner.client.apply_interactions(lookup, pending_transaction_id)?;
        let spru::client::apply_interactions::Ret {
            
        } = process_output(game_id, client_id, output, to_server, event_trigger);
        Ok(())
    })();

    event_trigger.trigger(super::event::ApplyInteractions::<Client> {
        game_id,
        client_id,
        pending_transaction_id,
        result,
        _client: PhantomData,
    });
}

pub(crate) fn revert_interactions<Client: super::ClientSSS>(
    game_id: common::component::GameId,
    client_id: super::component::ClientId,
    pending_transaction_id: Option<spru::transaction::Pending>,

    lookup: &mut super::BevyLookup<Client::State>, 
    mut runner: impl DerefMut<Target=super::component::Runner<Client>>,
    to_server: impl DerefMut<Target=super::component::ToServer<Client>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner.client.revert_interactions(lookup, pending_transaction_id)?;
        let spru::client::revert_interactions::Ret {
            
        } = process_output(game_id, client_id, output, to_server, event_trigger);
        Ok(())
    })();
    
    event_trigger.trigger(super::event::RevertInteractions::<Client> {
        game_id,
        client_id,
        pending_transaction_id,
        result,
        _client: PhantomData,
    });
}
