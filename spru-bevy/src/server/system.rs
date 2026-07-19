use bevy::prelude;

use crate::{common, local, server};

pub fn run_server<Server: server::ServerSSS>(
    mut commands: prelude::Commands,
    mut q_server: prelude::Query<(
        prelude::Entity,
        &crate::common::component::GameId,
        &mut server::component::Runner<Server>,
        &mut server::component::FromClient<Server>,
        &mut server::component::ToClient<Server>,
    )>,
) -> server::RunServerResult<()> {

    // TODO this should probably be done as async-compute since we don't touch bevy from the server:
    // https://bevy-cheatbook.github.io/fundamentals/async-compute.html
    for (
        entity,
        game_id,
        mut runner,
        mut from_client,
        mut to_client,
    ) in &mut q_server
    {
        if !from_client.is_empty() {
            prelude::trace!("[{game_id}] Server handling {} signals", from_client.len());
        }

        while let Some((sender, server_signal)) = from_client.dequeue_any() {
            let spru::server::Output {
                outbound,
                events,
                ret,
            } = runner.server.signal(sender, server_signal)?;

            to_client
                .enqueue_outbound(outbound);

            server::trigger_events::<Server>(entity, &mut commands, events);
            
            let _: () = ret;
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn create_save<Server: server::ServerSSS<PlayerInit: Clone, Reaction: Clone>>(
    game_id: &common::component::GameId,
    runner: &server::component::Runner<Server>,
) -> server::RunServerResult<spru::server::Save<Server>> {
    runner.server.save()?;
    let save = runner.server.save()?;

    prelude::debug!("[{game_id}] Server create save");

    Ok(save)
}
