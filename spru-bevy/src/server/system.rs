use std::{marker::PhantomData, ops::DerefMut};

use bevy::prelude;

use crate::common;

pub fn run_server<Server: crate::server::ServerSSS>(
    mut commands: prelude::Commands,
    mut q_server: prelude::Query<(
        prelude::Entity,
        &crate::common::component::GameId,
        &mut super::component::Runner<Server>,
        &mut super::component::FromClient<Server>,
        &mut super::component::ToClient<Server>,
        &mut super::component::FromUser<Server>,
        &mut super::component::PendingClients<Server>,
    )>,
) -> super::RunServerResult<()> {
    // TODO this should probably be done as async-compute since we don't touch bevy from the server:
    // https://bevy-cheatbook.github.io/fundamentals/async-compute.html
    for (
        _entity,
        game_id,
        mut runner,
        mut from_client,
        mut to_client,
        mut from_user,
        mut pending_clients,
    ) in &mut q_server
    {
        if !from_client.is_empty() {
            prelude::trace!("[{game_id}] Server handling {} signals", from_client.len());
        }

        while let Some((sender, server_signal)) = from_client.dequeue_any() {
            signal(
                *game_id,
                sender,
                server_signal,
                runner.reborrow(),
                to_client.reborrow(),
                &mut commands,
            );
        }

        while let Some(user_input) = from_user.dequeue() {
            match user_input {
                super::component::UserInput::AddPlayer(player_init_in) => {
                    add_player(
                        *game_id,
                        player_init_in,
                        runner.reborrow(),
                        to_client.reborrow(),
                        pending_clients.reborrow(),
                        &mut commands,
                    );
                }
                super::component::UserInput::ManualTrigger(trigger) => {
                    manual_trigger(
                        *game_id,
                        trigger,
                        runner.reborrow(),
                        to_client.reborrow(),
                        &mut commands,
                    );
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn init<Server: super::ServerSSS, GameInit>(
    game_init: GameInit,
    player_init: Server::PlayerInit,
    reaction: Server::Reaction,

    commands: &mut prelude::Commands,
) where
    GameInit: spru::game::Init<Action = Server::Action, Root = Server::Root>,
{
    let result = (|| {
        let server = Server::init(game_init, player_init, reaction)?;
        let root = common::component::Root::<Server::Common>::new(server.root().clone());
        let game_id = common::component::GameId::new(server.game_id());

        commands.spawn((
            prelude::Name::new(format!("[{:x}] spru server", game_id.friendly_display())),
            game_id,
            super::component::Runner::new(server),
            root,
        ));

        Ok(game_id)
    })();

    commands.trigger(super::event::Init::<Server> {
        result,
        _server: PhantomData,
    });
}

pub(crate) fn signal<Server: super::ServerSSS>(
    game_id: common::component::GameId,
    sender: spru::player::Id,
    signal: spru::common::signal::ToServer<Server::Common>,

    mut runner: impl DerefMut<Target = super::component::Runner<Server>>,
    to_client: impl DerefMut<Target = super::component::ToClient<Server>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner.server.signal(sender, signal)?;
        let () = process_output(game_id, output, to_client, event_trigger);
        Ok(())
    })();

    event_trigger.trigger(super::event::Signal::<Server> {
        game_id,
        sender,
        result,
        _server: PhantomData,
    });
}

pub(crate) fn add_player<Server: super::ServerSSS>(
    game_id: common::component::GameId,
    player_init_input: <Server::PlayerInit as spru::player::Init>::In,

    mut runner: impl DerefMut<Target = super::component::Runner<Server>>,
    to_client: impl DerefMut<Target = super::component::ToClient<Server>>,
    mut pending_clients: impl DerefMut<Target = super::component::PendingClients<Server>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner.server.add_player(player_init_input)?;
        let seed = process_output(game_id, output, to_client, event_trigger);
        let player_id = seed.local_player_id();

        pending_clients.enqueue(super::PendingClient { seed });

        Ok(player_id)
    })();

    event_trigger.trigger(super::event::AddPlayer::<Server> {
        game_id,
        result,
        _server: PhantomData,
    });
}

pub(crate) fn manual_trigger<Server: super::ServerSSS>(
    game_id: common::component::GameId,
    trigger: <Server::Reaction as spru::Reaction>::Trigger,

    mut runner: impl DerefMut<Target = super::component::Runner<Server>>,
    to_client: impl DerefMut<Target = super::component::ToClient<Server>>,
    event_trigger: &mut impl common::TriggerEvent,
) {
    let result = (|| {
        let output = runner.server.manual_trigger(trigger)?;
        let () = process_output(game_id, output, to_client, event_trigger);

        Ok(())
    })();

    event_trigger.trigger(super::event::ManualTrigger::<Server> {
        game_id,
        result,
        _server: PhantomData,
    });
}

#[allow(dead_code)]
pub(crate) fn create_save<Server: super::ServerSSS<PlayerInit: Clone, Reaction: Clone>>(
    game_id: &common::component::GameId,
    runner: &super::component::Runner<Server>,
) -> super::RunServerResult<spru::server::Save<Server>> {
    runner.server.save()?;
    let save = runner.server.save()?;

    prelude::debug!("[{game_id}] Server create save");

    Ok(save)
}

pub(crate) fn process_output<Server: super::ServerSSS, Ret>(
    game_id: common::component::GameId,
    output: spru::server::Output<Server, Ret>,
    mut to_client: impl std::ops::DerefMut<Target = super::component::ToClient<Server>>,
    event_trigger: &mut impl common::TriggerEvent,
) -> Ret {
    let spru::server::Output {
        outbound,
        events,
        ret,
    } = output;

    for (player_id, client_signal) in outbound {
        prelude::trace!("Signal to {player_id}");
        to_client.enqueue(player_id, client_signal);
    }

    for event in events {
        #[allow(clippy::single_match)]
        match event {
            spru::server::Event::GameComplete(game_complete) => {
                event_trigger.trigger(super::event::GameComplete::<Server> {
                    game_id,
                    game_outcome: game_complete.game_outcome,
                });
            }
            _ => {}
        }
    }

    ret
}
